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
  - [Features](#features)
    - [`debug-traces`](#debug-traces)
  - [Misc](#misc)
  - [Credits](#credits)
  - [License](#license)
    - [Code](#code)
    - [Assets](#assets)

## Quickstart

```
cargo add ghx_proc_gen
```

## Additions for Bevy users

Instead of using `ghx_proc_gen` directly, you can use `bevy_ghx_proc_gen` which exports `ghx_proc_gen` and some additional plugins & utilities dedicated to Bevy.
```
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

```
cargo run --example unicode-terrain
```

</details>

<details>
  <summary>Pillars example (using Bevy)</summary>

```
cargo run --example pillars
```

</details>


<details>
  <summary>Tile-layers example (using Bevy)</summary>

```
cargo run --example tile-layers
```

</details>

<details>
  <summary>Canyon example (using Bevy)</summary>

```
cargo run --example canyon
```

</details>

## Features

### `debug-traces`

Disabled by default, this feature will add many debug traces (using the `tracing` crate) to the core algorithm of the crate. Since some of those logs are on the hot path, the feature should only be enabled in debug.

When creating models, you can register a name for them with the `with_name` function. With the feature disabled, the function does nothing. But when enabled, the name of your models will be visible in the debug traces of the core algorithm, providing useful information about the current generation state.

The log level can be configured by the user crates (`tracing::level`, the `LogPlugin` for Bevy, ...).

![debug_traces](assets/debug_traces.png)

## Misc

Rules-writing tips:
 - Start simple, then add complexity iteratively
 - Changing the Node selection heuristic may drastically change the generated results
 - Diagonals constraints are harder and need intermediary models

Why "ghx" ?
- It serves as a namespace to avoid picking cargo names such as `proc_gen` or `bevy_proc_gen`

## Credits

Thanks to:
- Paul Merrel for the [Model Synthesis](https://paulmerrell.org/model-synthesis/) algorithm & implementation
- Maxim Gumin for the [Wave Function Collapse](https://github.com/mxgmn/WaveFunctionCollapse) algorithm & implementation
- BorisTheBrave for his C# library [DeBroglie](https://github.com/BorisTheBrave/DeBroglie) and the article series on his [website](https://www.boristhebrave.com/)

## License

### Code

ghx-proc-gen is free and open source. All code in this repository is dual-licensed under either:

* MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

### Assets

- Assets of the [`pillars`](bevy_examples/assets/pillars) and [`canyon`](bevy_examples/assets/canyon) examples were made for these examples by Gilles Henaux, and are availabe under [CC-BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/)
- Assets in the [`tile-layers`](bevy_examples/assets/tile_layers) example are "16x16 Game Assets" by George Bailey available on [OpenGameArt](https://opengameart.org/content/16x16-game-assets) under [CC-BY 4.0](https://creativecommons.org/licenses/by/4.0/)