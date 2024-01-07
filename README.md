[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)
[![crates.io](https://img.shields.io/crates/v/ghx_proc_gen)](https://crates.io/crates/ghx_proc_gen)

# Ghx Proc(edural) Gen(eneration)

A Rust library for 2D & 3D procedural generation with Model synthesis/Wave function Collapse, also available for the Bevy engine.

- [Ghx Proc(edural) Gen(eneration)](#ghx-procedural-geneneration)
  - [Quickstart](#quickstart)
  - [Additions for Bevy users](#additions-for-bevy-users)
    - [Bevy plugins](#bevy-plugins)
    - [Compatible Bevy versions](#compatible-bevy-versions)
  - [Examples](#examples)
  - [Misc](#misc)
  - [Credits](#credits)
  - [License](#license)

## Quickstart

```bash
cargo add ghx_proc_gen
```

## Additions for Bevy users

Instead of using `ghx_proc_gen` directly, you can use `bevy_ghx_proc_gen` which exports `ghx_proc_gen` and some addditional plugins & utilities dedicated to Bevy.
```bash
cargo add bevy_ghx_proc_gen
```

### Bevy plugins

- `GridDebugPlugin`
- `ProcGenExamplesPlugin`

### Compatible Bevy versions

Compatibility with Bevy versions:

| `bevy_ghx_proc_gen` | `bevy` |
| :------------------ | :----- |
| `0.1`               | `0.12` |

## Examples

<details>
  <summary>Terminal example</summary>

Todo

</details>

<details>
  <summary>Pillars example (using Bevy)</summary>

Todo

</details>


<details>
  <summary>Tile-layers example (using Bevy)</summary>

Todo

</details>

<details>
  <summary>Canyon example (using Bevy)</summary>

Todo

</details>

## Misc

Rules-writing tips:
 - Start simple and add complexity iteratively
 - Changing the Node selection heuristic may drastically change the generated result
 - Diagonals constraints are harder and need intermediary models

Why "ghx" ?
- It serves as a namespace to avoid picking cargo names such as `proc_gen` or `bevy_proc_gen`

## Credits

Thanks to:
- Paul Merrel for the [Model Synthesis](https://paulmerrell.org/model-synthesis/) algorithm & implementation
- Maxim Gumin for the [Wave Function Collapse](https://github.com/mxgmn/WaveFunctionCollapse) algorithm & implementation
- BorisTheBrave for his C# library [DeBroglie](https://github.com/BorisTheBrave/DeBroglie) and the article series on his [website](https://www.boristhebrave.com/)

## License

ghx-proc-gen is free and open source. All code in this repository is dual-licensed under either:

* MIT License ([LICENSE-MIT](docs/LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](docs/LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
