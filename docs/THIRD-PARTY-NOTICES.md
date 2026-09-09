# SkillDock 第三方依赖许可说明（Third-Party Notices）

生成日期：2026-09-09 · 生成方式：`python scripts/gen_notices.py`（Rust 经由 `cargo tree`；npm 经由 `package-lock.json`）· 票据：[#15](https://github.com/mrye111/SkillDock/issues/15)

本应用以静态链接方式分发 Rust 依赖、以打包方式分发前端依赖；未发现 GPL/LGPL/AGPL 类强 copyleft 依赖。

## 许可分布概览（去重后按包计数）

| 许可族 | 包数 |
| --- | --- |
| MIT | 217 |
| Apache-2.0 | 188 |
| Unicode-3.0 | 18 |
| ISC | 9 |
| BSD-3-Clause | 5 |
| MPL-2.0 | 5 |
| Zlib | 1 |
| (未标注) | 1 |
| CC-BY-4.0 | 1 |

## Rust 依赖清单

| 包（名称 版本） | 许可证 |
| --- | --- |
| `adler2 2.0.1` | 0BSD OR MIT OR Apache-2.0 |
| `ahash 0.8.12` | MIT OR Apache-2.0 |
| `aho-corasick 1.1.5` | Unlicense OR MIT |
| `alloc-no-stdlib 2.0.4` | BSD-3-Clause |
| `alloc-stdlib 0.2.4` | BSD-3-Clause |
| `anyhow 1.0.104` | MIT OR Apache-2.0 |
| `base64 0.22.1` | MIT OR Apache-2.0 |
| `base64 0.23.1` | MIT OR Apache-2.0 |
| `bit-set 0.8.0` | Apache-2.0 OR MIT |
| `bit-vec 0.8.0` | Apache-2.0 OR MIT |
| `bitflags 1.3.2` | MIT/Apache-2.0 |
| `bitflags 2.13.1` | MIT OR Apache-2.0 |
| `block-buffer 0.10.4` | MIT OR Apache-2.0 |
| `brotli 8.0.4` | BSD-3-Clause AND MIT |
| `brotli-decompressor 5.0.3` | BSD-3-Clause/MIT |
| `bstr 1.13.1` | MIT OR Apache-2.0 |
| `byteorder 1.5.0` | Unlicense OR MIT |
| `bytes 1.12.1` | MIT |
| `camino 1.2.5` | MIT OR Apache-2.0 |
| `cargo-platform 0.1.9` | MIT OR Apache-2.0 |
| `cargo_metadata 0.19.2` | MIT |
| `cfb 0.7.3` | MIT |
| `cfg-if 1.0.4` | MIT OR Apache-2.0 |
| `chrono 0.4.45` | MIT OR Apache-2.0 |
| `cookie 0.18.2` | MIT OR Apache-2.0 |
| `cpufeatures 0.2.17` | MIT OR Apache-2.0 |
| `crc32fast 1.5.1` | MIT OR Apache-2.0 |
| `crossbeam-channel 0.5.17` | MIT OR Apache-2.0 |
| `crossbeam-utils 0.8.23` | MIT OR Apache-2.0 |
| `crypto-common 0.1.7` | MIT OR Apache-2.0 |
| `cssparser 0.36.0` | MPL-2.0 |
| `cssparser-macros 0.6.1` | MPL-2.0 |
| `ctor 0.8.0` | Apache-2.0 OR MIT |
| `ctor-proc-macro 0.0.7` | Apache-2.0 OR MIT |
| `darling 0.24.1` | MIT |
| `darling_core 0.24.1` | MIT |
| `darling_macro 0.24.1` | MIT |
| `deranged 0.5.8` | MIT OR Apache-2.0 |
| `derive_more 2.1.1` | MIT |
| `derive_more-impl 2.1.1` | MIT |
| `digest 0.10.7` | MIT OR Apache-2.0 |
| `dirs 6.0.0` | MIT OR Apache-2.0 |
| `dirs-sys 0.5.0` | MIT OR Apache-2.0 |
| `displaydoc 0.2.7` | MIT OR Apache-2.0 |
| `dom_query 0.27.0` | MIT |
| `dpi 0.1.2` | Apache-2.0 AND MIT |
| `dtoa 1.0.11` | MIT OR Apache-2.0 |
| `dtoa-short 0.3.5` | MPL-2.0 |
| `dunce 1.0.5` | CC0-1.0 OR MIT-0 OR Apache-2.0 |
| `dyn-clone 1.0.20` | MIT OR Apache-2.0 |
| `equivalent 1.0.2` | Apache-2.0 OR MIT |
| `erased-serde 0.4.10` | MIT OR Apache-2.0 |
| `fallible-iterator 0.3.0` | MIT/Apache-2.0 |
| `fallible-streaming-iterator 0.1.9` | MIT/Apache-2.0 |
| `fastrand 2.5.0` | Apache-2.0 OR MIT |
| `fdeflate 0.3.7` | MIT OR Apache-2.0 |
| `flate2 1.1.10` | MIT OR Apache-2.0 |
| `fnv 1.0.7` | Apache-2.0 / MIT |
| `foldhash 0.2.0` | Zlib |
| `form_urlencoded 1.2.2` | MIT OR Apache-2.0 |
| `generic-array 0.14.7` | MIT |
| `getrandom 0.3.4` | MIT OR Apache-2.0 |
| `getrandom 0.4.3` | MIT OR Apache-2.0 |
| `glob 0.3.4` | MIT OR Apache-2.0 |
| `globset 0.4.20` | Unlicense OR MIT |
| `hashbrown 0.12.3` | MIT OR Apache-2.0 |
| `hashbrown 0.14.5` | MIT OR Apache-2.0 |
| `hashbrown 0.17.1` | MIT OR Apache-2.0 |
| `hashlink 0.9.1` | MIT OR Apache-2.0 |
| `heck 0.5.0` | MIT OR Apache-2.0 |
| `hex 0.4.3` | MIT OR Apache-2.0 |
| `html5ever 0.38.0` | MIT OR Apache-2.0 |
| `http 1.5.0` | MIT OR Apache-2.0 |
| `ico 0.5.0` | MIT |
| `icu_collections 2.3.0` | Unicode-3.0 |
| `icu_locale_core 2.3.0` | Unicode-3.0 |
| `icu_normalizer 2.3.0` | Unicode-3.0 |
| `icu_normalizer_data 2.3.0` | Unicode-3.0 |
| `icu_properties 2.3.0` | Unicode-3.0 |
| `icu_properties_data 2.3.0` | Unicode-3.0 |
| `icu_provider 2.3.1` | Unicode-3.0 |
| `ident_case 1.0.1` | MIT/Apache-2.0 |
| `idna 1.1.0` | MIT OR Apache-2.0 |
| `idna_adapter 1.2.2` | Apache-2.0 OR MIT |
| `indexmap 1.9.3` | Apache-2.0 OR MIT |
| `indexmap 2.14.2` | Apache-2.0 OR MIT |
| `infer 0.19.0` | MIT |
| `itoa 1.0.18` | MIT OR Apache-2.0 |
| `json-patch 3.0.1` | MIT/Apache-2.0 |
| `jsonptr 0.6.3` | MIT OR Apache-2.0 |
| `keyboard-types 0.7.0` | MIT OR Apache-2.0 |
| `libc 0.2.189` | MIT OR Apache-2.0 |
| `libsqlite3-sys 0.30.1` | MIT |
| `litemap 0.8.3` | Unicode-3.0 |
| `lock_api 0.4.14` | MIT OR Apache-2.0 |
| `log 0.4.34` | MIT OR Apache-2.0 |
| `markup5ever 0.38.0` | MIT OR Apache-2.0 |
| `memchr 2.8.3` | Unlicense OR MIT |
| `mime 0.3.17` | MIT OR Apache-2.0 |
| `miniz_oxide 0.8.9` | MIT OR Zlib OR Apache-2.0 |
| `miniz_oxide 0.9.1` | MIT OR Zlib OR Apache-2.0 |
| `mio 1.2.3` | MIT |
| `muda 0.19.3` | Apache-2.0 OR MIT |
| `new_debug_unreachable 1.0.6` | MIT |
| `num-conv 0.2.2` | MIT OR Apache-2.0 |
| `num-traits 0.2.19` | MIT OR Apache-2.0 |
| `once_cell 1.21.4` | MIT OR Apache-2.0 |
| `option-ext 0.2.0` | MPL-2.0 |
| `parking_lot 0.12.5` | MIT OR Apache-2.0 |
| `parking_lot_core 0.9.12` | MIT OR Apache-2.0 |
| `percent-encoding 2.3.2` | MIT OR Apache-2.0 |
| `phf 0.13.1` | MIT |
| `phf_generator 0.13.1` | MIT |
| `phf_macros 0.13.1` | MIT |
| `phf_shared 0.13.1` | MIT |
| `pin-project-lite 0.2.17` | Apache-2.0 OR MIT |
| `plist 1.10.1` | MIT |
| `png 0.17.16` | MIT OR Apache-2.0 |
| `potential_utf 0.1.6` | Unicode-3.0 |
| `powerfmt 0.2.0` | MIT OR Apache-2.0 |
| `precomputed-hash 0.1.1` | MIT |
| `proc-macro2 1.0.107` | MIT OR Apache-2.0 |
| `quick-xml 0.42.0` | MIT |
| `quote 1.0.47` | MIT OR Apache-2.0 |
| `raw-window-handle 0.6.2` | MIT OR Apache-2.0 OR Zlib |
| `regex 1.13.1` | MIT OR Apache-2.0 |
| `regex-automata 0.4.18` | MIT OR Apache-2.0 |
| `regex-syntax 0.8.11` | MIT OR Apache-2.0 |
| `rfd 0.16.0` | MIT |
| `rusqlite 0.32.1` | MIT |
| `rustc-hash 2.1.3` | Apache-2.0 OR MIT |
| `ryu 1.0.23` | Apache-2.0 OR BSL-1.0 |
| `same-file 1.0.6` | Unlicense/MIT |
| `schemars 0.8.22` | MIT |
| `schemars_derive 0.8.22` | MIT |
| `scopeguard 1.2.0` | MIT OR Apache-2.0 |
| `selectors 0.36.1` | MPL-2.0 |
| `semver 1.0.28` | MIT OR Apache-2.0 |
| `serde 1.0.229` | MIT OR Apache-2.0 |
| `serde-untagged 0.1.9` | MIT OR Apache-2.0 |
| `serde_core 1.0.229` | MIT OR Apache-2.0 |
| `serde_derive 1.0.229` | MIT OR Apache-2.0 |
| `serde_derive_internals 0.29.1` | MIT OR Apache-2.0 |
| `serde_json 1.0.151` | MIT OR Apache-2.0 |
| `serde_repr 0.1.21` | MIT OR Apache-2.0 |
| `serde_spanned 1.1.1` | MIT OR Apache-2.0 |
| `serde_with 3.23.0` | MIT OR Apache-2.0 |
| `serde_with_macros 3.23.0` | MIT OR Apache-2.0 |
| `serde_yaml 0.9.34+deprecated` | MIT OR Apache-2.0 |
| `serialize-to-javascript 0.1.2` | MIT OR Apache-2.0 |
| `serialize-to-javascript-impl 0.1.2` | MIT OR Apache-2.0 |
| `servo_arc 0.4.3` | MIT OR Apache-2.0 |
| `sha2 0.10.9` | MIT OR Apache-2.0 |
| `simd-adler32 0.3.10` | MIT |
| `siphasher 1.0.3` | MIT/Apache-2.0 |
| `skilldock 0.1.0` |  |
| `smallvec 1.16.0` | MIT OR Apache-2.0 |
| `socket2 0.6.5` | MIT OR Apache-2.0 |
| `softbuffer 0.4.8` | MIT OR Apache-2.0 |
| `stable_deref_trait 1.2.1` | MIT OR Apache-2.0 |
| `string_cache 0.9.0` | MIT OR Apache-2.0 |
| `strsim 0.11.1` | MIT |
| `syn 2.0.119` | MIT OR Apache-2.0 |
| `syn 3.0.5` | MIT OR Apache-2.0 |
| `synstructure 0.13.2` | MIT |
| `tao 0.35.3` | Apache-2.0 |
| `tauri 2.11.5` | Apache-2.0 OR MIT |
| `tauri-codegen 2.6.3` | Apache-2.0 OR MIT |
| `tauri-macros 2.6.3` | Apache-2.0 OR MIT |
| `tauri-plugin-dialog 2.7.3` | Apache-2.0 OR MIT |
| `tauri-plugin-fs 2.5.2` | Apache-2.0 OR MIT |
| `tauri-plugin-single-instance 2.4.4` | Apache-2.0 OR MIT |
| `tauri-plugin-window-state 2.4.1` | Apache-2.0 OR MIT |
| `tauri-runtime 2.11.3` | Apache-2.0 OR MIT |
| `tauri-runtime-wry 2.11.4` | Apache-2.0 OR MIT |
| `tauri-utils 2.9.3` | Apache-2.0 OR MIT |
| `tendril 0.5.1` | MIT OR Apache-2.0 |
| `thiserror 1.0.69` | MIT OR Apache-2.0 |
| `thiserror 2.0.20` | MIT OR Apache-2.0 |
| `thiserror-impl 1.0.69` | MIT OR Apache-2.0 |
| `thiserror-impl 2.0.20` | MIT OR Apache-2.0 |
| `time 0.3.55` | MIT OR Apache-2.0 |
| `time-core 0.1.9` | MIT OR Apache-2.0 |
| `time-macros 0.2.32` | MIT OR Apache-2.0 |
| `tinystr 0.8.4` | Unicode-3.0 |
| `tokio 1.53.1` | MIT |
| `toml 1.1.5+spec-1.1.0` | MIT OR Apache-2.0 |
| `toml_datetime 1.1.1+spec-1.1.0` | MIT OR Apache-2.0 |
| `toml_parser 1.1.3+spec-1.1.0` | MIT OR Apache-2.0 |
| `toml_writer 1.1.2+spec-1.1.0` | MIT OR Apache-2.0 |
| `tracing 0.1.44` | MIT |
| `tracing-attributes 0.1.31` | MIT |
| `tracing-core 0.1.36` | MIT |
| `typeid 1.0.3` | MIT OR Apache-2.0 |
| `typenum 1.20.1` | MIT OR Apache-2.0 |
| `unic-char-property 0.9.0` | MIT/Apache-2.0 |
| `unic-char-range 0.9.0` | MIT/Apache-2.0 |
| `unic-common 0.9.0` | MIT/Apache-2.0 |
| `unic-ucd-ident 0.9.0` | MIT/Apache-2.0 |
| `unic-ucd-version 0.9.0` | MIT/Apache-2.0 |
| `unicode-ident 1.0.24` | (MIT OR Apache-2.0) AND Unicode-3.0 |
| `unicode-segmentation 1.13.3` | MIT OR Apache-2.0 |
| `unsafe-libyaml 0.2.11` | MIT |
| `url 2.5.8` | MIT OR Apache-2.0 |
| `urlpattern 0.3.0` | MIT |
| `utf8_iter 1.0.4` | Apache-2.0 OR MIT |
| `uuid 1.26.0` | Apache-2.0 OR MIT |
| `walkdir 2.5.0` | Unlicense/MIT |
| `web_atoms 0.2.6` | MIT OR Apache-2.0 |
| `webview2-com 0.38.2` | MIT |
| `webview2-com-macros 0.8.1` | MIT |
| `webview2-com-sys 0.38.2` | MIT |
| `winapi-util 0.1.11` | Unlicense OR MIT |
| `window-vibrancy 0.6.0` | Apache-2.0 OR MIT |
| `windows 0.61.3` | MIT OR Apache-2.0 |
| `windows-collections 0.2.0` | MIT OR Apache-2.0 |
| `windows-core 0.61.2` | MIT OR Apache-2.0 |
| `windows-future 0.2.1` | MIT OR Apache-2.0 |
| `windows-implement 0.60.2` | MIT OR Apache-2.0 |
| `windows-interface 0.59.3` | MIT OR Apache-2.0 |
| `windows-link 0.1.3` | MIT OR Apache-2.0 |
| `windows-link 0.2.1` | MIT OR Apache-2.0 |
| `windows-numerics 0.2.0` | MIT OR Apache-2.0 |
| `windows-result 0.3.4` | MIT OR Apache-2.0 |
| `windows-strings 0.4.2` | MIT OR Apache-2.0 |
| `windows-sys 0.59.0` | MIT OR Apache-2.0 |
| `windows-sys 0.60.2` | MIT OR Apache-2.0 |
| `windows-sys 0.61.2` | MIT OR Apache-2.0 |
| `windows-targets 0.52.6` | MIT OR Apache-2.0 |
| `windows-targets 0.53.5` | MIT OR Apache-2.0 |
| `windows-threading 0.1.0` | MIT OR Apache-2.0 |
| `windows-version 0.1.7` | MIT OR Apache-2.0 |
| `windows_x86_64_msvc 0.52.6` | MIT OR Apache-2.0 |
| `windows_x86_64_msvc 0.53.1` | MIT OR Apache-2.0 |
| `winnow 1.0.4` | MIT |
| `writeable 0.6.4` | Unicode-3.0 |
| `wry 0.55.1` | Apache-2.0 OR MIT |
| `yoke 0.8.3` | Unicode-3.0 |
| `yoke-derive 0.8.2` | Unicode-3.0 |
| `zerocopy 0.8.56` | BSD-2-Clause OR Apache-2.0 OR MIT |
| `zerofrom 0.1.8` | Unicode-3.0 |
| `zerofrom-derive 0.1.7` | Unicode-3.0 |
| `zerotrie 0.2.5` | Unicode-3.0 |
| `zerovec 0.11.8` | Unicode-3.0 |
| `zerovec-derive 0.11.6` | Unicode-3.0 |
| `zmij 1.0.23` | MIT |

## npm 依赖清单

| 包（名称 版本） | 许可证 |
| --- | --- |
| `@alloc/quick-lru 5.3.0` | MIT |
| `@babel/code-frame 7.29.7` | MIT |
| `@babel/compat-data 7.29.7` | MIT |
| `@babel/core 7.29.7` | MIT |
| `@babel/generator 7.29.8` | MIT |
| `@babel/helper-compilation-targets 7.29.7` | MIT |
| `@babel/helper-globals 7.29.7` | MIT |
| `@babel/helper-module-imports 7.29.7` | MIT |
| `@babel/helper-module-transforms 7.29.7` | MIT |
| `@babel/helper-plugin-utils 7.29.7` | MIT |
| `@babel/helper-string-parser 7.29.7` | MIT |
| `@babel/helper-validator-identifier 7.29.7` | MIT |
| `@babel/helper-validator-option 7.29.7` | MIT |
| `@babel/helpers 7.29.7` | MIT |
| `@babel/parser 7.29.8` | MIT |
| `@babel/plugin-transform-react-jsx-self 7.29.7` | MIT |
| `@babel/plugin-transform-react-jsx-source 7.29.7` | MIT |
| `@babel/template 7.29.7` | MIT |
| `@babel/traverse 7.29.8` | MIT |
| `@babel/types 7.29.8` | MIT |
| `@esbuild/aix-ppc64 0.25.12` | MIT |
| `@esbuild/android-arm 0.25.12` | MIT |
| `@esbuild/android-arm64 0.25.12` | MIT |
| `@esbuild/android-x64 0.25.12` | MIT |
| `@esbuild/darwin-arm64 0.25.12` | MIT |
| `@esbuild/darwin-x64 0.25.12` | MIT |
| `@esbuild/freebsd-arm64 0.25.12` | MIT |
| `@esbuild/freebsd-x64 0.25.12` | MIT |
| `@esbuild/linux-arm 0.25.12` | MIT |
| `@esbuild/linux-arm64 0.25.12` | MIT |
| `@esbuild/linux-ia32 0.25.12` | MIT |
| `@esbuild/linux-loong64 0.25.12` | MIT |
| `@esbuild/linux-mips64el 0.25.12` | MIT |
| `@esbuild/linux-ppc64 0.25.12` | MIT |
| `@esbuild/linux-riscv64 0.25.12` | MIT |
| `@esbuild/linux-s390x 0.25.12` | MIT |
| `@esbuild/linux-x64 0.25.12` | MIT |
| `@esbuild/netbsd-arm64 0.25.12` | MIT |
| `@esbuild/netbsd-x64 0.25.12` | MIT |
| `@esbuild/openbsd-arm64 0.25.12` | MIT |
| `@esbuild/openbsd-x64 0.25.12` | MIT |
| `@esbuild/openharmony-arm64 0.25.12` | MIT |
| `@esbuild/sunos-x64 0.25.12` | MIT |
| `@esbuild/win32-arm64 0.25.12` | MIT |
| `@esbuild/win32-ia32 0.25.12` | MIT |
| `@esbuild/win32-x64 0.25.12` | MIT |
| `@jridgewell/gen-mapping 0.3.13` | MIT |
| `@jridgewell/remapping 2.3.5` | MIT |
| `@jridgewell/resolve-uri 3.1.2` | MIT |
| `@jridgewell/sourcemap-codec 1.6.0` | MIT |
| `@jridgewell/trace-mapping 0.3.31` | MIT |
| `@napi-rs/lzma-linux-x64-gnu 1.5.1` | MIT |
| `@nodelib/fs.scandir 2.1.5` | MIT |
| `@nodelib/fs.stat 2.0.5` | MIT |
| `@nodelib/fs.walk 1.2.8` | MIT |
| `@rolldown/pluginutils 1.0.0-beta.27` | MIT |
| `@rollup/rollup-android-arm-eabi 4.63.1` | MIT |
| `@rollup/rollup-android-arm64 4.63.1` | MIT |
| `@rollup/rollup-darwin-arm64 4.63.1` | MIT |
| `@rollup/rollup-darwin-x64 4.63.1` | MIT |
| `@rollup/rollup-freebsd-arm64 4.63.1` | MIT |
| `@rollup/rollup-freebsd-x64 4.63.1` | MIT |
| `@rollup/rollup-linux-arm-gnueabihf 4.63.1` | MIT |
| `@rollup/rollup-linux-arm-musleabihf 4.63.1` | MIT |
| `@rollup/rollup-linux-arm64-gnu 4.63.1` | MIT |
| `@rollup/rollup-linux-arm64-musl 4.63.1` | MIT |
| `@rollup/rollup-linux-loong64-gnu 4.63.1` | MIT |
| `@rollup/rollup-linux-loong64-musl 4.63.1` | MIT |
| `@rollup/rollup-linux-ppc64-gnu 4.63.1` | MIT |
| `@rollup/rollup-linux-ppc64-musl 4.63.1` | MIT |
| `@rollup/rollup-linux-riscv64-gnu 4.63.1` | MIT |
| `@rollup/rollup-linux-riscv64-musl 4.63.1` | MIT |
| `@rollup/rollup-linux-s390x-gnu 4.63.1` | MIT |
| `@rollup/rollup-linux-x64-gnu 4.63.1` | MIT |
| `@rollup/rollup-linux-x64-musl 4.63.1` | MIT |
| `@rollup/rollup-openbsd-x64 4.63.1` | MIT |
| `@rollup/rollup-openharmony-arm64 4.63.1` | MIT |
| `@rollup/rollup-win32-arm64-msvc 4.63.1` | MIT |
| `@rollup/rollup-win32-ia32-msvc 4.63.1` | MIT |
| `@rollup/rollup-win32-x64-gnu 4.63.1` | MIT |
| `@rollup/rollup-win32-x64-msvc 4.63.1` | MIT |
| `@tauri-apps/api 2.11.1` | Apache-2.0 OR MIT |
| `@tauri-apps/cli 2.11.4` | Apache-2.0 OR MIT |
| `@tauri-apps/cli-darwin-arm64 2.11.4` | Apache-2.0 OR MIT |
| `@tauri-apps/cli-darwin-x64 2.11.4` | Apache-2.0 OR MIT |
| `@tauri-apps/cli-linux-arm-gnueabihf 2.11.4` | Apache-2.0 OR MIT |
| `@tauri-apps/cli-linux-arm64-gnu 2.11.4` | Apache-2.0 OR MIT |
| `@tauri-apps/cli-linux-arm64-musl 2.11.4` | Apache-2.0 OR MIT |
| `@tauri-apps/cli-linux-riscv64-gnu 2.11.4` | Apache-2.0 OR MIT |
| `@tauri-apps/cli-linux-x64-gnu 2.11.4` | Apache-2.0 OR MIT |
| `@tauri-apps/cli-linux-x64-musl 2.11.4` | Apache-2.0 OR MIT |
| `@tauri-apps/cli-win32-arm64-msvc 2.11.4` | Apache-2.0 OR MIT |
| `@tauri-apps/cli-win32-ia32-msvc 2.11.4` | Apache-2.0 OR MIT |
| `@tauri-apps/cli-win32-x64-msvc 2.11.4` | Apache-2.0 OR MIT |
| `@tauri-apps/plugin-dialog 2.7.3` | MIT OR Apache-2.0 |
| `@types/babel__core 7.20.5` | MIT |
| `@types/babel__generator 7.27.0` | MIT |
| `@types/babel__template 7.4.4` | MIT |
| `@types/babel__traverse 7.28.0` | MIT |
| `@types/estree 1.0.9` | MIT |
| `@types/node 22.20.1` | MIT |
| `@types/prop-types 15.7.15` | MIT |
| `@types/react 18.3.31` | MIT |
| `@types/react-dom 18.3.7` | MIT |
| `@vitejs/plugin-react 4.7.0` | MIT |
| `any-promise 1.3.0` | MIT |
| `anymatch 3.1.3` | ISC |
| `arg 5.0.2` | MIT |
| `autoprefixer 10.5.5` | MIT |
| `baseline-browser-mapping 2.11.21` | Apache-2.0 |
| `binary-extensions 2.3.0` | MIT |
| `braces 3.0.3` | MIT |
| `browserslist 4.28.9` | MIT |
| `camelcase-css 2.0.1` | MIT |
| `caniuse-lite 1.0.30001810` | CC-BY-4.0 |
| `chokidar 3.6.0` | MIT |
| `clsx 2.1.1` | MIT |
| `commander 4.1.1` | MIT |
| `convert-source-map 2.0.0` | MIT |
| `cssesc 3.0.0` | MIT |
| `csstype 3.2.3` | MIT |
| `debug 4.4.3` | MIT |
| `didyoumean 1.2.2` | Apache-2.0 |
| `dlv 1.1.3` | MIT |
| `electron-to-chromium 1.5.423` | ISC |
| `es-errors 1.3.0` | MIT |
| `esbuild 0.25.12` | MIT |
| `escalade 3.2.0` | MIT |
| `fast-glob 3.3.3` | MIT |
| `fastq 1.20.3` | ISC |
| `fill-range 7.1.1` | MIT |
| `fraction.js 5.3.4` | MIT |
| `fsevents 2.3.3` | MIT |
| `function-bind 1.1.2` | MIT |
| `gensync 1.0.0-beta.2` | MIT |
| `glob-parent 6.0.2` | ISC |
| `hasown 2.0.4` | MIT |
| `is-binary-path 2.1.0` | MIT |
| `is-core-module 2.16.2` | MIT |
| `is-extglob 2.1.1` | MIT |
| `is-glob 4.0.3` | MIT |
| `is-number 7.0.0` | MIT |
| `jiti 1.21.7` | MIT |
| `js-tokens 4.0.0` | MIT |
| `jsesc 3.1.0` | MIT |
| `json5 2.2.3` | MIT |
| `lilconfig 3.1.3` | MIT |
| `lines-and-columns 1.2.4` | MIT |
| `loose-envify 1.4.0` | MIT |
| `lru-cache 5.1.1` | ISC |
| `lucide-react 0.475.0` | ISC |
| `merge2 1.4.1` | MIT |
| `micromatch 4.0.8` | MIT |
| `ms 2.1.3` | MIT |
| `mz 2.7.0` | MIT |
| `nanoid 3.3.18` | MIT |
| `node-releases 2.0.54` | MIT |
| `normalize-path 3.0.0` | MIT |
| `object-assign 4.1.1` | MIT |
| `object-hash 3.0.0` | MIT |
| `path-parse 1.0.7` | MIT |
| `picocolors 1.1.1` | ISC |
| `picomatch 2.3.2` | MIT |
| `pirates 4.0.7` | MIT |
| `postcss 8.5.28` | MIT |
| `postcss-import 15.1.0` | MIT |
| `postcss-js 4.1.0` | MIT |
| `postcss-load-config 6.0.1` | MIT |
| `postcss-nested 6.2.0` | MIT |
| `postcss-selector-parser 6.1.4` | MIT |
| `postcss-value-parser 4.2.0` | MIT |
| `queue-microtask 1.2.3` | MIT |
| `react 18.3.1` | MIT |
| `react-dom 18.3.1` | MIT |
| `react-refresh 0.17.0` | MIT |
| `read-cache 1.0.2` | MIT |
| `readdirp 3.6.0` | MIT |
| `resolve 1.22.12` | MIT |
| `reusify 1.1.0` | MIT |
| `rollup 4.63.1` | MIT |
| `run-parallel 1.2.0` | MIT |
| `scheduler 0.23.2` | MIT |
| `semver 6.3.1` | ISC |
| `source-map-js 1.2.1` | BSD-3-Clause |
| `sucrase 3.35.1` | MIT |
| `supports-preserve-symlinks-flag 1.0.0` | MIT |
| `tailwind-merge 3.6.0` | MIT |
| `tailwindcss 3.4.19` | MIT |
| `thenify 3.3.1` | MIT |
| `thenify-all 1.6.0` | MIT |
| `tinyglobby 0.2.17` | MIT |
| `to-regex-range 5.0.1` | MIT |
| `ts-interface-checker 0.1.13` | Apache-2.0 |
| `typescript 5.7.3` | Apache-2.0 |
| `undici-types 6.21.0` | MIT |
| `update-browserslist-db 1.3.2` | MIT |
| `util-deprecate 1.0.2` | MIT |
| `vite 6.4.3` | MIT |
| `yallist 3.1.1` | ISC |

## 许可证文本与要点

### MIT License
```
MIT License

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```

### Apache License 2.0
Apache License, Version 2.0

Full canonical text: https://www.apache.org/licenses/LICENSE-2.0.txt

Key terms: permission to use, reproduce, prepare derivative works, publicly display/perform, sublicense and distribute, subject to (4.1) including a copy of this License, (4.2) marking modified files, (4.3) retaining notices, (4.4) including NOTICE file contents. Includes express patent grant (section 3) and trademark limitation (section 6).

### ISC License
```
ISC License

Permission to use, copy, modify, and/or distribute this software for any purpose with or without fee is hereby granted, provided that the above copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
```

### BSD 3-Clause License
```
BSD 3-Clause License

Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer.
2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution.
3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote products derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

### Mozilla Public License 2.0（适用于 cssparser/selectors/dtoa-short/option-ext 等）
Mozilla Public License, v. 2.0

Full canonical text: https://www.mozilla.org/MPL/2.0/

File-level weak copyleft: Covered Software files must remain under MPL-2.0 with source made available; may be combined into a Larger Work under different terms as long as MPL-covered files stay under MPL-2.0 (sections 3.1/3.3).

### Unicode License V3（适用于 icu_*、litemap、tinystr、zerotrie 等 ICU4X 组件）
UNICODE LICENSE V3

Permission is hereby granted, free of charge, to any person obtaining a copy of data files and any associated documentation or computer programs, to deal without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, and/or sell copies, provided that (a) this copyright and permission notice appear with all copies, (b) the license is available at https://www.unicode.org/license.txt, and (c) modified files carry prominent notices stating the modifications.

### The Unlicense
The Unlicense

This is free and unencumbered software released into the public domain.
Anyone is free to copy, modify, publish, use, compile, sell, or distribute this software, either in source code form or as a compiled binary, for any purpose, commercial or non-commercial, and by any means.
In jurisdictions that recognize copyright laws, the author or authors of this software dedicate any and all copyright interest in the software to the public domain.
THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND. Full text: https://unlicense.org/

### zlib License
zlib License

This software is provided 'as-is', without any express or implied warranty. In no event will the authors be held liable for any damages arising from the use of this software. Permission is granted to anyone to use this software for any purpose, including commercial applications, and to alter it and redistribute it freely, subject to the following restrictions: 1. The origin of this software must not be misrepresented; 2. Altered source versions must be plainly marked as such; 3. This notice may not be removed or altered from any source distribution.

### Boost Software License 1.0（适用于 ryu）
Boost Software License - Version 1.0

Permission is hereby granted, free of charge, to any person or organization obtaining a copy of the software and accompanying documentation covered by this license (the "Software") to use, reproduce, display, distribute, execute, and transmit the Software, and to prepare derivative works of the Software, and to permit third-parties to whom the Software is furnished to do so, all subject to the following: The copyright notices in the Software and this entire statement must be included in all copies of the Software, in whole or in part. THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND. Full text: https://www.boost.org/LICENSE_1_0.txt

### 0BSD License
0BSD License

Permission to use, copy, modify, and/or distribute this software for any purpose with or without fee is hereby granted. THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES. Full text: https://opensource.org/license/0bsd

### MIT No Attribution（MIT-0）
MIT No Attribution (MIT-0)

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software. THE SOFTWARE IS PROVIDED "AS IS". Full text: https://opensource.org/license/mit-0

### CC0 1.0 Universal
CC0 1.0 Universal

The person who associated a work with this deed has dedicated the work to the public domain by waiving all of his or her rights to the work worldwide under copyright law, including all related and neighboring rights, to the extent allowed by law. Full text: https://creativecommons.org/publicdomain/zero/1.0/legalcode

### CC-BY-4.0
Creative Commons Attribution 4.0 International (CC-BY-4.0)

Applies to: caniuse-lite (browser support data, npm). Requirement: attribution to https://caniuse.com with a link to the license. Full text: https://creativecommons.org/licenses/by/4.0/legalcode

## Microsoft WebView2 Runtime

本应用运行依赖 **Microsoft Edge WebView2 Runtime**（专有组件，非本应用代码）：
- 标准安装包以 downloadBootstrapper 方式在缺失时引导下载（https://go.microsoft.com/fwlink/?linkid=2124701）；离线安装包内嵌微软官方离线安装组件。
- WebView2 按其自身许可条款分发：**Microsoft Software License Terms: Microsoft Edge WebView2 Runtime**（随安装组件展示；参考 https://learn.microsoft.com/microsoft-edge/webview2/）。
