<p align="center">
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="256" height="256">
  <rect width="256" height="256" rx="16" fill="#111"/>
  <!-- struct frame - a bracket showing mapped layout -->
  <rect x="56" y="40" width="144" height="176" rx="8" fill="none" stroke="#334155" stroke-width="2"/>
  <!-- field slots at specific offsets -->
  <rect x="72" y="56" width="112" height="24" rx="4" fill="#f97316"/>
  <rect x="72" y="92" width="112" height="24" rx="4" fill="#38bdf8"/>
  <rect x="72" y="128" width="112" height="24" rx="4" fill="#38bdf8"/>
  <!-- pointer chain field -->
  <rect x="72" y="164" width="72" height="24" rx="4" fill="#a78bfa"/>
  <!-- pointer arrow from chain field to external target -->
  <line x1="144" y1="176" x2="176" y2="176" stroke="#a78bfa" stroke-width="2" stroke-linecap="round"/>
  <line x1="176" y1="176" x2="176" y2="144" stroke="#a78bfa" stroke-width="2" stroke-linecap="round"/>
  <line x1="176" y1="144" x2="216" y2="144" stroke="#a78bfa" stroke-width="2" stroke-linecap="round"/>
  <circle cx="216" cy="144" r="4" fill="#a78bfa"/>
  <!-- offset tick marks on the left -->
  <line x1="44" y1="68" x2="56" y2="68" stroke="#555" stroke-width="1"/>
  <line x1="44" y1="104" x2="56" y2="104" stroke="#555" stroke-width="1"/>
  <line x1="44" y1="140" x2="56" y2="140" stroke="#555" stroke-width="1"/>
  <line x1="44" y1="176" x2="56" y2="176" stroke="#555" stroke-width="1"/>
</svg>
</p>

<h1 align="center">procmod-layout</h1>

<p align="center">Struct mapping with pointer chain traversal via derive macros.</p>

---

Map remote process memory into Rust structs. Declare byte offsets on each field, optionally follow pointer chains through multiple indirections, and read the entire struct in one call. Built on [procmod-core](https://github.com/procmod/procmod-core).

## Install

```toml
[dependencies]
procmod-layout = "1"
```

## Quick start

Read a game's player state from a known base address:

```rust
use procmod_layout::{GameStruct, Process};

#[derive(GameStruct)]
struct Player {
    #[offset(0x100)]
    health: f32,
    #[offset(0x104)]
    max_health: f32,
    #[offset(0x108)]
    position: [f32; 3],
}

fn main() -> procmod_layout::Result<()> {
    let game = Process::attach(pid)?;
    let player = Player::read(&game, player_base)?;
    println!("hp: {}/{}", player.health, player.max_health);
    println!("pos: {:?}", player.position);
    Ok(())
}
```

## Usage

### Basic struct mapping

Every field needs an `#[offset(N)]` attribute specifying its byte offset from the base address. The derive macro generates a `read(process, base) -> Result<Self>` method.

```rust
use procmod_layout::{GameStruct, Process};

#[derive(GameStruct)]
struct GameSettings {
    #[offset(0x00)]
    difficulty: u32,
    #[offset(0x04)]
    volume: f32,
    #[offset(0x08)]
    fov: f32,
    #[offset(0x10)]
    mouse_sensitivity: f64,
}
```

### Pointer chains

When a value is behind one or more pointer indirections, use `#[pointer_chain(...)]` to follow the chain automatically. The offsets list the intermediate dereference offsets before reading the final value.

For example, reading a damage multiplier stored behind two pointer hops:

```rust
use procmod_layout::{GameStruct, Process};

#[derive(GameStruct)]
struct CombatState {
    #[offset(0x50)]
    is_attacking: bool,
    #[offset(0x54)]
    combo_count: u32,
    #[offset(0x60)]
    #[pointer_chain(0x10, 0x08)]
    damage_mult: f32,
}

// The pointer chain for damage_mult reads:
//   1. read pointer at (base + 0x60)
//   2. read pointer at (ptr + 0x10)
//   3. read f32 at (ptr + 0x08)
```

### Composing with procmod-scan

Use [procmod-scan](https://github.com/procmod/procmod-scan) to find a structure's base address after a game update, then read it with a layout:

```rust
use procmod_layout::{GameStruct, Process};
use procmod_scan::Pattern;

#[derive(GameStruct)]
struct Inventory {
    #[offset(0x00)]
    slot_count: u32,
    #[offset(0x04)]
    gold: u32,
    #[offset(0x10)]
    weight: f32,
}

fn find_inventory(process: &Process, module: &[u8], module_base: usize) -> procmod_layout::Result<Inventory> {
    let sig = Pattern::from_ida("48 8D 0D ? ? ? ? E8 ? ? ? ? 48 8B D8").unwrap();
    let offset = sig.scan_first(module).expect("inventory signature not found");
    let base = module_base + offset;
    Inventory::read(process, base)
}
```

## Supported types

Any type that is `Copy` and valid for any bit pattern works as a field type:

- Primitives: `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`, `f32`, `f64`, `bool`, `usize`
- Fixed-size arrays: `[f32; 3]`, `[u8; 16]`, etc.
- Any `#[repr(C)]` struct that is `Copy`

## License

MIT
