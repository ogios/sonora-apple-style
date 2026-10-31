# Sonora GPUI patches

Source: https://github.com/sonorahq/gpui at 7eea897d4765f59dc9dac1ca468a91e84bb518a6.
The source workspace, assets and Apache license are vendored here. The root Cargo
manifest patches this git source to local crates; gpui and gpui_platform retain
the same pinned revision. No Cargo registry or git checkout is modified.

- gpui_wgpu: initialize Vulkan alone, test adapter/device/surface configuration,
  then initialize GL only if Vulkan fails. Device recovery uses the same sequence.
  Shared windows reuse their existing instance and device.
- gpui: reserve one decoration run per shaped line; grow normally for styled text.
