# Standalone server runtime

`sonora-server` owns music state on the Tokio service runtime. Its dependency graph excludes
GPUI, native browser windows, fonts, icons and renderers. The GUI executes browser requests
received over IPC; returned credentials are acknowledged and are absent from published state.

## Measurement

Measured on Linux on 2026-11-04 with release binaries, comparing the saved server executable
from before this change with the standalone runtime. Each of three runs used a fresh private
configuration, data, cache and socket directory, no accounts, no display, no session bus, an
ALSA null output and one subscribed protocol client. After five seconds, three memory samples
were taken half a second apart. The table reports the median of the three runs.

| Metric | Previous executable | Standalone runtime |
| --- | ---: | ---: |
| RSS | 40.16 MiB | 40.68 MiB |
| PSS | 36.57 MiB | 37.10 MiB |
| Anonymous PSS | 6.67 MiB | 5.18 MiB |
| Threads | 42 | 14 |
| GPUI workers | 24 | 0 |

Total idle RSS and PSS are effectively unchanged in this workload. Anonymous memory and thread
counts decrease, and the server no longer initializes a graphics or font platform. These
idle measurements alone do not establish a memory saving for a signed-in account, network
playback, large libraries or the combined GUI and server workload.

## Paced local playback

Measured on the same Linux machine on 2026-11-04, using PipeWire's PulseAudio socket and
release executables. The saved server uses protocol version 2; the standalone server and GUI
use version 3. Both server executables and the GUI were copied into the same temporary
filesystem before measurement. The GUI ran as a real Wayland window in its default view.

Each configuration ran twice, in forward then reverse order: previous server alone,
standalone server alone, and standalone server with GUI. Every run had fresh configuration,
data, cache, state and IPC directories, no signed-in accounts, no session bus, and one
subscribed measurement client. Servers had no display environment. Local-file lyrics and
Discord presence were disabled. System PipeWire processes were shared infrastructure and
were excluded from the totals.

The script generated a 90-second silent WAV (48 kHz, stereo, 16-bit PCM, no embedded cover).
It waited 15 seconds after launch, measured idle memory for 5 seconds, then sent
`playback.open_paths` and measured for 60 seconds. Every 200 milliseconds it read RSS and
PSS from `/proc/<pid>/smaps_rollup`; every second it checked playback state, progress and
the selected PulseAudio output. All runs remained `Playing` after startup and advanced to
approximately 59 seconds of audio over the measurement. No child processes were observed.
The temporary windows and servers were stopped after their runs.

Values below are MiB. Idle and steady values are medians of the two run medians. Steady
state is the last 20 seconds of playback; peak is the largest sample across both runs,
including playback startup. A sampled peak can miss a transient shorter than 200 ms.

| Workload / process | Idle RSS | Playback steady RSS | Playback peak RSS |
| --- | ---: | ---: | ---: |
| Previous server, alone | 43.69 | 46.21 | 46.77 |
| Standalone server, alone | 36.33 | 38.95 | 39.17 |
| Standalone server, with GUI | 47.33 | 48.94 | 50.40 |
| GUI, with standalone server | 231.48 | 232.56 | 236.09 |
| Standalone server + GUI total | 278.81 | 281.50 | 285.71 |

Totals sum processes at each sample timestamp; the total peak need not equal the sum of
the individual peaks. Summed RSS counts shared resident pages in each process. It is not a
measurement of unique physical memory. Attaching the GUI also creates service read models,
so the server-alone result cannot substitute for the server in the combined workload.

The individual runs' total RSS values were:

| Run | Idle | Playback steady | Playback peak |
| --- | ---: | ---: | ---: |
| Previous server 1 | 43.262 | 45.652 | 45.660 |
| Standalone server 1 | 36.184 | 38.723 | 38.723 |
| Standalone server + GUI 1 | 282.965 | 285.277 | 285.711 |
| Standalone server + GUI 2 | 274.656 | 277.730 | 277.730 |
| Standalone server 2 | 36.480 | 39.172 | 39.172 |
| Previous server 2 | 44.121 | 46.762 | 46.766 |

The server-alone playback steady RSS decreased by 7.26 MiB (15.7%). The paired decreases
were 6.93 and 7.59 MiB. Idle RSS decreased by 7.36 MiB in this setup; playback added about
2.52 MiB before and 2.62 MiB after. The reduction is therefore chiefly resident overhead,
rather than a reduction in playback's incremental buffers. These baselines differ from the
earlier ALSA-null experiment, which also used a shorter warmup, so its approximately 40 MiB
values are not used to calculate this saving.

The combined standalone server and GUI increased by about 2.69 MiB from idle to playback.
There is no saved previous GUI executable, so there is no controlled before/after total for
the complete application. This local, unsigned-in workload does not establish the saving for
NetEase streaming, populated libraries, online lyrics or cover-heavy views, and cannot be
compared directly with the original application's approximately 400 MiB screenshot.

Reproduce with the saved previous executable and the release binaries:

```sh
python3 docs/performance/measure-playback-memory.py \
  --before /tmp/sonora-server-memory-pv74dp2d/baseline-server \
  --after target/release/sonora-server \
  --gui target/release/sonora \
  --pulse-server unix:/run/user/1000/pulse/native \
  --seconds 60 --repetitions 2 \
  --output /tmp/sonora-playback-memory-repeat
```

Use a new output directory per invocation. The previous executable must be retained to
repeat the before/after comparison. Binary hashes, per-process summaries and last playback
positions are recorded in [the result summary](playback-memory-2026-11-04.json).
The full timestamped RSS/PSS samples and logs from this run are in
`/tmp/sonora-playback-memory-20261104/results.json` and its run subdirectories.

## Verification

- Check the server dependency tree for every target with `cargo tree -p service --target all`.
- Run the existing workspace tests and Clippy with all targets and warnings denied.
- Verify local file playback, pause, seek, ordered subscriptions, page release and shutdown in
  an isolated configuration.
- Verify browser request ownership, cancellation on GUI disconnect and GUI startup over IPC.
