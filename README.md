# XIVData Oxidizer

This is a Rust project that uses [ironworks](https://github.com/ackwell/ironworks) and [EXDSchema](https://github.com/xivdev/EXDSchema) to extract Final Fantasy XIV game data as CSV or JSON files. It is a spiritual successor to [SaintCoinach.Cmd](https://github.com/xivapi/SaintCoinach) and provides functionality similar to its `rawexd` command.

Game data is parsed using the schemas provided by EXDSchema which is included as a git submodule. Because the schemas are cloned locally, you can easily make changes to them and extract game data without waiting for the upstream schemas to update.

## Just want the data?

Check out [xiv-data](https://github.com/skyborn-industries/xiv-data).

## Requirements

- Rust 1.87
- A local installation of FFXIV

## Setup

```
git clone --recurse-submodules https://github.com/skyborn-industries/xiv-data-oxidizer
cd xiv-data-oxidizer
```

## Usage

```
git submodule update --remote
cargo run -- "C:\Program Files (x86)\Square Enix\FINAL FANTASY XIV - A Realm Reborn"
```

CSV is the default export format. To export JSON instead:

```
cargo run -- "C:\Program Files (x86)\Square Enix\FINAL FANTASY XIV - A Realm Reborn" --format json
```

By default, media exports use English (`en`). To export data and localized media fields for a specific language:

```
cargo run -- "C:\Program Files (x86)\Square Enix\FINAL FANTASY XIV - A Realm Reborn" --lang de --maps --bgm
```

Supported language codes are `en`, `de`, `fr`, `ja`, `chs`, `ko`, and `tc`.

To also extract the icons referenced by schema fields marked as `type: icon`:

```
cargo run -- "C:\Program Files (x86)\Square Enix\FINAL FANTASY XIV - A Realm Reborn" --images
```

Use `--hd-images` to prefer SaintCoinach-style HD icon textures (`_hr1`) when available. Extracted images are written as WebP files under `output/images`.

To export BGM referenced by the `BGM` and `OrchestrionPath` sheets:

```
cargo run -- "C:\Program Files (x86)\Square Enix\FINAL FANTASY XIV - A Realm Reborn" --bgm
```

This decodes SCD entries to `.ogg` or `.wav` files under `output/bgm`.

To export ULD textures, provide a local path list like SaintCoinach's `PathList.gz` or a text file containing `ui/uld/...tex` paths:

```
cargo run -- "C:\Program Files (x86)\Square Enix\FINAL FANTASY XIV - A Realm Reborn" --uld "C:\path\to\PathList.gz"
```

If no path list is provided, `--uld` downloads and caches SaintCoinach's default `PathList.gz` for one day at `output/cache/PathList.gz`.

ULD textures are written as WebP files under `output/uld`.

To export map images and loading images:

```
cargo run -- "C:\Program Files (x86)\Square Enix\FINAL FANTASY XIV - A Realm Reborn" --maps --loading-images
```

Map images are written as WebP files under `output/maps`, and loading images are written as WebP files under `output/loadingimage`.

---

FINAL FANTASY is a registered trademark of Square Enix Holdings Co., Ltd.

FINAL FANTASY XIV © SQUARE ENIX CO., LTD.
