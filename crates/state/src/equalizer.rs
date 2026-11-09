//! The equalizer the app plays through: the curve every engine reads, the profiles the settings
//! store it in, undo and redo, the sound each device rule asks for, and the clip warning the
//! output meter raises against it.

use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::{App, Context, Entity, Subscription, Task};
use music::Meter;
use music::equalizer::{Curve, Edit, EqualizerWriter, PREAMP_RANGE, Preamp, Preset, Tone, apo};
use music::output::OutputId;

use crate::overload::{OutputLevel, Overload};
use crate::{AppSettings, Outputs, Playback, Reloaded};

/// How long after the last glided edit the curve is stored, so a drag or a run of wheel steps
/// reaches the settings once, when it pauses.
const GLIDE_STORE: Duration = Duration::from_millis(300);
/// How many equalizer edits can be undone.
const UNDO_STEPS: usize = 50;

/// The equalizer as it plays. Created once in `Sonora`, it hands every engine the same reader
/// and meter, so a change reaches the output without a restart. It follows the output the sound
/// plays on and plays the sound that output's device rule names.
pub struct Equalizer {
    settings: Entity<AppSettings>,
    writer: EqualizerWriter,
    /// Shared with every engine like the equalizer's reader, and read only while watched.
    meter: Meter,
    overload: Overload,
    /// The output's level as the meter last read it, notified on its own.
    output_level: Entity<OutputLevel>,
    /// Stores a glided curve in the settings once the edits pause.
    glide_store: Option<Task<()>>,
    /// Stores a glided tone in the settings once the edits pause.
    tone_store: Option<Task<()>>,
    history: History,
    /// The id of the output played on as `Outputs` last settled it, so a rule added for it can
    /// play at once and the same output again is not a move.
    output: Option<String>,
    /// The volume as playback last had it, so a change to it takes the clip warning down.
    volume: f32,
    _subscriptions: Vec<Subscription>,
}

/// A sound a device rule plays: a built-in preset, loaded into the unnamed profile the way
/// picking it does, or a stored profile by its name. The unnamed profile is never one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sound {
    Preset(Preset),
    Profile(String),
}

/// Which output a device rule is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleDevice {
    /// One output, by the key `Outputs::binding_key` gives it. Its name is only shown.
    Output(OutputId),
    /// Every output no other rule names.
    Other,
}

/// The sound to play once the sound moves to a device. A device has one rule at most.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceRule {
    pub device: RuleDevice,
    pub sound: Sound,
}

/// The playing profile's curves from before each edit and from before each undo, newest last.
/// It is kept in memory only and emptied when another profile plays.
#[derive(Default)]
struct History {
    undo: Vec<Curve>,
    redo: Vec<Curve>,
}

impl RuleDevice {
    /// The output's key, or `None` for every other output. Two rules are for the same device
    /// when their keys are equal, whatever names they were seen with.
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Output(output) => Some(&output.id),
            Self::Other => None,
        }
    }
}

impl History {
    /// Remembers `curve`, the one an edit is about to replace, and forgets what was undone.
    fn record(&mut self, curve: &Curve) {
        if self.undo.len() == UNDO_STEPS {
            self.undo.remove(0);
        }
        self.undo.push(curve.clone());
        self.redo.clear();
    }

    /// Steps back from `current` and returns the curve to play instead.
    fn undo(&mut self, current: &Curve) -> Option<Curve> {
        let curve = self.undo.pop()?;
        self.redo.push(current.clone());
        Some(curve)
    }

    /// Steps forward from `current` again and returns the curve to play instead.
    fn redo(&mut self, current: &Curve) -> Option<Curve> {
        let curve = self.redo.pop()?;
        self.undo.push(current.clone());
        Some(curve)
    }
}

impl Equalizer {
    /// Plays what the settings hold. It follows no output until `follow` is called, since
    /// `Outputs` is built after the playback that reads this equalizer.
    pub fn new(settings: Entity<AppSettings>, cx: &mut Context<Self>) -> Self {
        let reloaded = cx.subscribe(&settings, |this, _, _: &Reloaded, cx| {
            // The file's curve and tone win over glides that were still to be stored.
            this.glide_store = None;
            this.tone_store = None;
            this.follow_settings(cx)
        });
        let mut writer = EqualizerWriter::new(
            settings.read(cx).equalizer(),
            settings.read(cx).equalizer_curve(),
        );
        writer.set_tone(settings.read(cx).equalizer_tone(), Edit::Replace);
        Self {
            settings,
            writer,
            meter: Meter::default(),
            overload: Overload::default(),
            output_level: cx.new(|_| OutputLevel::default()),
            glide_store: None,
            tone_store: None,
            history: History::default(),
            output: None,
            volume: 0.,
            _subscriptions: vec![reloaded],
        }
    }

    /// Starts following the output `outputs` plays on, and the volume of `playback`, which
    /// takes the clip warning down when it changes.
    pub(crate) fn follow(
        &mut self,
        playback: &Entity<Playback>,
        outputs: &Entity<Outputs>,
        cx: &mut Context<Self>,
    ) {
        self.volume = playback.read(cx).volume();
        self._subscriptions
            .push(cx.observe(playback, |this, playback, cx| {
                let volume = playback.read(cx).volume();
                if volume != this.volume {
                    this.volume = volume;
                    this.overload.settle(Instant::now());
                }
            }));
        self._subscriptions
            .push(cx.observe(outputs, |this, outputs, cx| {
                let playing = outputs.read(cx).binding_key().map(|output| output.id);
                if let Some(id) = playing {
                    this.follow_output(id, cx);
                }
            }));
    }

    /// The reader every engine plays the curve through.
    pub(crate) fn reader(&self) -> music::Equalizer {
        self.writer.reader()
    }

    /// The meter every engine's output reports its loudest sample to.
    pub(crate) fn meter(&self) -> Meter {
        self.meter.clone()
    }

    pub fn enabled(&self) -> bool {
        self.writer.enabled()
    }

    /// The playing sound's own curve, the one profiles store and the filter list edits.
    pub fn curve(&self) -> &Curve {
        self.writer.curve()
    }

    /// The bass and treble laid over every sound.
    pub fn tone(&self) -> Tone {
        self.writer.tone()
    }

    /// The curve heard: the sound's own with the tone's shelves after it, and the preamp that
    /// makes room for both.
    pub fn played(&self) -> &Curve {
        self.writer.played()
    }

    /// Puts `tone` in place and stores it, the engines cross-fading to it. It turns the equalizer
    /// on like picking a sound does, since a tone is only heard through it.
    pub fn set_tone(&mut self, tone: Tone, cx: &mut Context<Self>) {
        self.tone_store = None;
        self.writer.set_tone(tone, Edit::Replace);
        self.toned(cx);
        self.store_tone(cx);
    }

    /// Puts `tone` in place the way a slider drag does: the engines glide to it, and the settings
    /// store it once the edits pause.
    pub fn glide_tone(&mut self, tone: Tone, cx: &mut Context<Self>) {
        if tone == self.writer.tone() {
            return;
        }
        self.writer.set_tone(tone, Edit::Glide);
        self.toned(cx);
        self.tone_store = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(GLIDE_STORE).await;
            this.update(cx, |this, cx| {
                this.tone_store = None;
                this.store_tone(cx);
            })
            .ok();
        }));
    }

    /// Turns the equalizer on or off. The engines pick the change up on their next frame and
    /// fade to it, so no restart and no click.
    pub fn set_enabled(&mut self, on: bool, cx: &mut Context<Self>) {
        self.writer.set_enabled(on);
        self.overload.settle(Instant::now());
        self.settings
            .update(cx, |settings, cx| settings.set_equalizer(on, cx));
        cx.notify();
    }

    /// Puts a whole curve in place of the playing profile's, the way a preset pick or a typed
    /// value does: the engines cross-fade to it.
    pub fn set_curve(&mut self, curve: Curve, cx: &mut Context<Self>) {
        if curve == *self.writer.curve() {
            return;
        }
        self.history.record(self.writer.curve());
        self.replace_curve(curve, cx);
    }

    /// Whether there is an edit to undo, and whether there is an undo to redo.
    pub fn history(&self) -> (bool, bool) {
        (!self.history.undo.is_empty(), !self.history.redo.is_empty())
    }

    /// Plays the curve from before the last edit, cross-fading to it.
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if let Some(curve) = self.history.undo(self.writer.curve()) {
            self.replace_curve(curve, cx);
        }
    }

    /// Plays the curve the last undo took away again.
    pub fn redo(&mut self, cx: &mut Context<Self>) {
        if let Some(curve) = self.history.redo(self.writer.curve()) {
            self.replace_curve(curve, cx);
        }
    }

    /// Puts a curve in place that moves one filter a little from the playing one, the way a
    /// drag does: the engines glide it there. The settings store it once the edits pause.
    pub fn glide_curve(&mut self, curve: Curve, cx: &mut Context<Self>) {
        if curve == *self.writer.curve() {
            return;
        }
        // A glide waiting to be stored belongs to the gesture that started it.
        if self.glide_store.is_none() {
            self.history.record(self.writer.curve());
        }
        self.writer.set_curve(curve, Edit::Glide);
        self.overload.settle(Instant::now());
        self.glide_store = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(GLIDE_STORE).await;
            this.update(cx, |this, cx| this.store_glided(cx)).ok();
        }));
        cx.notify();
    }

    /// The preamp the curve heard resolves to, in decibels, with room for the tone's boost.
    pub fn preamp_db(&self) -> f64 {
        self.writer.preamp_db()
    }

    /// How many decibels over full scale the clip warning says the output went, to a tenth,
    /// while something watches. It changes at most about once a second, and the equalizer
    /// notifies only when it does.
    pub fn overload(&self) -> Option<f64> {
        self.overload.db()
    }

    /// The preamp "Lower preamp" would set, or `None` without an overload or when the preamp
    /// is as low as it goes.
    pub fn lowered_preamp(&self) -> Option<f64> {
        self.overload.lowered(self.writer.preamp_db())
    }

    /// The output's recent level. It notifies on its own about ten times a second while the
    /// meter is watched, so a level meter observes it rather than the equalizer.
    pub fn output_level(&self) -> Entity<OutputLevel> {
        self.output_level.clone()
    }

    /// Starts the level meter and the clip warning afresh, from no overload, for a screen that
    /// starts watching. `Signal::watch` then reads the meter on its own tick.
    pub(crate) fn reset_meter(&mut self, cx: &mut Context<Self>) {
        self.overload = Overload::default();
        self.meter.take();
        self.output_level.update(cx, |level, cx| level.reset(cx));
        cx.notify();
    }

    /// Reads the meter once, the loudest sample since the read before, feeding the level meter
    /// and the clip warning. It notifies only when the warning changed.
    pub(crate) fn read_meter(&mut self, cx: &mut Context<Self>) {
        let (peak, now) = (self.meter.take(), Instant::now());
        self.output_level
            .update(cx, |level, cx| level.observe(peak, now, cx));
        if self.overload.observe(peak, now) {
            cx.notify();
        }
    }

    /// Lowers the preamp by the overload and a margin, made manual, so what overloaded fits. The
    /// sound stores its own preamp, which the tone lowers further, so the stored one keeps the
    /// difference between the two.
    pub fn lower_preamp(&mut self, cx: &mut Context<Self>) {
        let Some(preamp) = self.lowered_preamp() else {
            return;
        };
        let own = self.writer.curve().preamp_db() - self.writer.preamp_db();
        let preamp = (preamp + own).clamp(*PREAMP_RANGE.start(), *PREAMP_RANGE.end());
        let filters = self.writer.curve().filters().to_vec();
        if let Ok(curve) = Curve::new(Preamp::Manual(preamp), filters) {
            self.set_curve(curve, cx);
        }
    }

    /// Adds `rule` in place of the one its device had. When it is the rule the output played on
    /// now goes by, its sound plays at once.
    pub fn set_rule(&mut self, rule: DeviceRule, cx: &mut Context<Self>) {
        let device = rule.device.clone();
        self.settings
            .update(cx, |settings, cx| settings.set_equalizer_rule(rule, cx));
        let governs = self.output.as_deref().is_some_and(|id| {
            self.settings
                .read(cx)
                .equalizer_rule_for_output(id)
                .is_some_and(|rule| rule.device.id() == device.id())
        });
        if governs {
            self.play_output_sound(cx);
        }
    }

    /// Removes the rule for `device`. What plays stays.
    pub fn remove_rule(&mut self, device: &RuleDevice, cx: &mut Context<Self>) {
        self.settings.update(cx, |settings, cx| {
            settings.remove_equalizer_rule(device, cx)
        });
    }

    /// Plays `sound` and turns the equalizer on, the way picking it does. A profile that is no
    /// longer stored plays nothing.
    pub fn play_sound(&mut self, sound: &Sound, cx: &mut Context<Self>) {
        if self.load_sound(sound, cx) && !self.enabled() {
            self.set_enabled(true, cx);
        }
    }

    /// Plays profile `index`, cross-fading to its curve.
    pub fn select_profile(&mut self, index: usize, cx: &mut Context<Self>) {
        self.store_glided(cx);
        self.settings.update(cx, |settings, cx| {
            settings.select_equalizer_profile(index, cx)
        });
        self.follow_settings(cx);
    }

    /// Plays profile `index` and turns the equalizer on, the way picking a sound does.
    pub fn play_profile(&mut self, index: usize, cx: &mut Context<Self>) {
        self.select_profile(index, cx);
        if !self.enabled() {
            self.set_enabled(true, cx);
        }
    }

    /// Plays `preset` in the profile with no name, switching to it first when another plays, and
    /// turns the equalizer on. A named profile is never overwritten.
    pub fn play_preset(&mut self, preset: Preset, cx: &mut Context<Self>) {
        self.load_preset(preset, cx);
        if !self.enabled() {
            self.set_enabled(true, cx);
        }
    }

    /// Puts `preset` in the profile with no name and plays that profile, leaving the equalizer on
    /// or off.
    fn load_preset(&mut self, preset: Preset, cx: &mut Context<Self>) {
        let curve = preset.curve();
        let settings = self.settings.read(cx);
        if settings.equalizer_scratch() != Some(settings.equalizer_profile()) {
            self.store_glided(cx);
            self.settings.update(cx, |settings, cx| {
                settings.select_equalizer_scratch(&curve, cx)
            });
            self.follow_settings(cx);
        }
        self.set_curve(curve, cx);
    }

    /// Switches to `sound` and leaves the equalizer on or off. A stored profile that plays
    /// already keeps its undo history. Returns false when `sound` names no stored profile.
    fn load_sound(&mut self, sound: &Sound, cx: &mut Context<Self>) -> bool {
        match sound {
            Sound::Preset(preset) => self.load_preset(*preset, cx),
            Sound::Profile(name) => {
                let settings = self.settings.read(cx);
                let Some(index) = settings.equalizer_profile_named(name) else {
                    return false;
                };
                if index != settings.equalizer_profile() {
                    self.select_profile(index, cx);
                }
            }
        }
        true
    }

    /// Adds a profile named after `name` holding `curve` and plays it. `AppSettings` makes the
    /// name unique.
    pub fn add_profile(&mut self, name: &str, curve: Curve, cx: &mut Context<Self>) {
        self.store_glided(cx);
        self.settings.update(cx, |settings, cx| {
            settings.add_equalizer_profile(name, &curve, cx)
        });
        self.follow_settings(cx);
    }

    /// Saves the playing curve as a new profile named after `name` and plays it. When the curve
    /// was in the profile with no name, that profile goes back to flat. `AppSettings` makes the
    /// name unique.
    pub fn save_as(&mut self, name: &str, cx: &mut Context<Self>) {
        self.store_glided(cx);
        let curve = self.writer.curve().clone();
        self.settings.update(cx, |settings, cx| {
            if settings.equalizer_scratch() == Some(settings.equalizer_profile()) {
                settings.set_equalizer_curve(&Preset::Flat.curve(), cx);
            }
            settings.add_equalizer_profile(name, &curve, cx);
        });
        self.follow_settings(cx);
    }

    /// Removes profile `index` while it is still the one named `name`, unless it is the only
    /// one, and the rules that play it. When it was playing, the sound the output's rule names
    /// plays, else the profile `AppSettings` picks.
    pub fn remove_profile(&mut self, index: usize, name: &str, cx: &mut Context<Self>) {
        self.store_glided(cx);
        let playing = self.settings.read(cx).equalizer_profile() == index;
        self.settings.update(cx, |settings, cx| {
            settings.remove_equalizer_profile(index, name, cx)
        });
        self.follow_settings(cx);
        if let Some(sound) = self.output_sound(cx).filter(|_| playing) {
            self.load_sound(&sound, cx);
        }
    }

    /// Reads an Equalizer APO or AutoEQ text into a new profile named after `name` and plays
    /// it. Returns the lines it left out, or why nothing was imported.
    pub fn import(
        &mut self,
        name: &str,
        text: &str,
        cx: &mut Context<Self>,
    ) -> Result<Vec<apo::Skipped>, apo::Refused> {
        let parsed = apo::parse(text)?;
        self.add_profile(name, parsed.curve, cx);
        Ok(parsed.skipped)
    }

    /// The playing curve as an Equalizer APO text.
    pub fn export(&self) -> String {
        apo::export(self.writer.curve())
    }

    /// Follows a tone that was just put in place: the clip warning starts afresh, and the
    /// equalizer turns on when it was off.
    fn toned(&mut self, cx: &mut Context<Self>) {
        self.overload.settle(Instant::now());
        match self.enabled() {
            true => cx.notify(),
            false => self.set_enabled(true, cx),
        }
    }

    fn store_tone(&mut self, cx: &mut Context<Self>) {
        let tone = self.writer.tone();
        self.settings
            .update(cx, |settings, cx| settings.set_equalizer_tone(tone, cx));
    }

    fn replace_curve(&mut self, curve: Curve, cx: &mut Context<Self>) {
        self.writer.set_curve(curve, Edit::Replace);
        self.overload.settle(Instant::now());
        self.save_curve(cx);
    }

    /// Stores a glided curve that waits for the edits to pause now, so a profile change that
    /// follows keeps it in the profile it was made in.
    fn store_glided(&mut self, cx: &mut Context<Self>) {
        if self.glide_store.take().is_some() {
            self.save_curve(cx);
        }
    }

    /// Takes in output `id` as the one played on, and plays the sound its rule names when it is
    /// another output than the last one.
    fn follow_output(&mut self, id: String, cx: &mut Context<Self>) {
        if self.output.as_ref() == Some(&id) {
            return;
        }
        self.output = Some(id);
        self.play_output_sound(cx);
    }

    /// The sound the output played on asks for: its own rule's, else the one for any other
    /// device.
    fn output_sound(&self, cx: &App) -> Option<Sound> {
        let id = self.output.as_deref()?;
        let rule = self.settings.read(cx).equalizer_rule_for_output(id)?;
        Some(rule.sound.clone())
    }

    /// Plays the sound the output played on asks for and turns the equalizer on. Nothing changes
    /// when no rule applies.
    fn play_output_sound(&mut self, cx: &mut Context<Self>) {
        let Some(sound) = self.output_sound(cx) else {
            return;
        };
        log::info!("equalizer: playing {sound:?}, the sound for the output played on");
        self.play_sound(&sound, cx);
    }

    /// Plays what the settings say, the equalizer on or off and the selected profile's curve,
    /// as one replace, so the sound, the picker and the screen agree after a profile change. A
    /// tone still waiting to be stored is newer than the settings' and stays.
    fn follow_settings(&mut self, cx: &mut Context<Self>) {
        let settings = self.settings.read(cx);
        let (on, curve) = (settings.equalizer(), settings.equalizer_curve());
        if self.tone_store.is_none() {
            self.writer
                .set_tone(settings.equalizer_tone(), Edit::Replace);
        }
        self.history = History::default();
        self.overload.settle(Instant::now());
        // Whichever change is heard goes last, so only one replace reaches the ear.
        match on {
            true => {
                self.writer.set_curve(curve, Edit::Replace);
                self.writer.set_enabled(true);
            }
            false => {
                self.writer.set_enabled(false);
                self.writer.set_curve(curve, Edit::Replace);
            }
        }
        cx.notify();
    }

    fn save_curve(&mut self, cx: &mut Context<Self>) {
        self.glide_store = None;
        let curve = self.writer.curve().clone();
        self.settings
            .update(cx, |settings, cx| settings.set_equalizer_curve(&curve, cx));
        cx.notify();
    }
}
