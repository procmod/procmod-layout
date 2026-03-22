//! Struct mapping with pointer chain traversal via derive macros.
//!
//! Map remote process memory into Rust structs using `#[derive(GameStruct)]`.
//! Each field declares its byte offset from a base address. Fields can follow
//! pointer chains through multiple indirections before reading the final value.
//!
//! Built on top of [procmod-core](https://crates.io/crates/procmod-core) for
//! cross-platform memory access.
//!
//! # Example
//!
//! ```ignore
//! use procmod_layout::{GameStruct, Process};
//!
//! #[derive(GameStruct)]
//! struct Player {
//!     #[offset(0x100)]
//!     health: f32,
//!     #[offset(0x104)]
//!     max_health: f32,
//!     #[offset(0x200)]
//!     #[pointer_chain(0x10, 0x8)]
//!     damage_mult: f32,
//! }
//!
//! let process = Process::attach(pid)?;
//! let player = Player::read(&process, base_address)?;
//! println!("hp: {}/{}", player.health, player.max_health);
//! ```

// allows the derive macro's generated `::procmod_layout::...` paths to resolve
// when this crate is being compiled (including in tests)
extern crate self as procmod_layout;

pub use procmod_core::{Error, Process, Result};
pub use procmod_layout_derive::GameStruct;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(GameStruct, Debug)]
    struct SimpleLayout {
        #[offset(0)]
        a: u32,
        #[offset(4)]
        b: u32,
    }

    #[derive(GameStruct, Debug)]
    struct WithGap {
        #[offset(0)]
        first: u64,
        #[offset(16)]
        second: u64,
    }

    #[derive(GameStruct, Debug)]
    struct SingleField {
        #[offset(0)]
        value: f32,
    }

    #[derive(GameStruct, Debug)]
    struct ArrayField {
        #[offset(0)]
        values: [u32; 4],
    }

    #[derive(GameStruct, Debug)]
    struct MixedTypes {
        #[offset(0)]
        byte_val: u8,
        #[offset(4)]
        int_val: u32,
        #[offset(8)]
        float_val: f64,
    }

    #[derive(GameStruct, Debug)]
    struct WithPointerChain {
        #[offset(0)]
        direct: u32,
        #[offset(8)]
        #[pointer_chain(0)]
        through_ptr: u32,
    }

    fn self_process() -> Process {
        let pid = std::process::id();
        Process::attach(pid).expect("failed to attach to self")
    }

    #[test]
    fn read_simple_layout() {
        let data: [u32; 2] = [42, 99];
        let process = self_process();
        let base = data.as_ptr() as usize;
        let result = SimpleLayout::read(&process, base).unwrap();
        assert_eq!(result.a, 42);
        assert_eq!(result.b, 99);
    }

    #[test]
    fn read_with_gap() {
        let mut buf = [0u8; 24];
        let first: u64 = 0xDEAD_BEEF;
        let second: u64 = 0xCAFE_BABE;
        buf[0..8].copy_from_slice(&first.to_ne_bytes());
        buf[16..24].copy_from_slice(&second.to_ne_bytes());

        let process = self_process();
        let base = buf.as_ptr() as usize;
        let result = WithGap::read(&process, base).unwrap();
        assert_eq!(result.first, 0xDEAD_BEEF);
        assert_eq!(result.second, 0xCAFE_BABE);
    }

    #[test]
    fn read_single_field() {
        let value: f32 = 3.14;
        let process = self_process();
        let base = &value as *const f32 as usize;
        let result = SingleField::read(&process, base).unwrap();
        assert!((result.value - 3.14).abs() < f32::EPSILON);
    }

    #[test]
    fn read_array_field() {
        let data: [u32; 4] = [10, 20, 30, 40];
        let process = self_process();
        let base = data.as_ptr() as usize;
        let result = ArrayField::read(&process, base).unwrap();
        assert_eq!(result.values, [10, 20, 30, 40]);
    }

    #[test]
    fn read_mixed_types() {
        let mut buf = [0u8; 16];
        buf[0] = 0xFF;
        buf[4..8].copy_from_slice(&42u32.to_ne_bytes());
        buf[8..16].copy_from_slice(&2.718f64.to_ne_bytes());

        let process = self_process();
        let base = buf.as_ptr() as usize;
        let result = MixedTypes::read(&process, base).unwrap();
        assert_eq!(result.byte_val, 0xFF);
        assert_eq!(result.int_val, 42);
        assert!((result.float_val - 2.718).abs() < f64::EPSILON);
    }

    #[test]
    fn read_pointer_chain() {
        let target: u32 = 12345;
        let target_ptr: usize = &target as *const u32 as usize;

        // buf layout:
        //   offset 0: direct u32 (999)
        //   offset 4: padding
        //   offset 8: pointer to target_ptr (which points to target)
        let mut buf = [0u8; 16];
        buf[0..4].copy_from_slice(&999u32.to_ne_bytes());
        buf[8..8 + std::mem::size_of::<usize>()].copy_from_slice(&target_ptr.to_ne_bytes());

        let process = self_process();
        let base = buf.as_ptr() as usize;
        let result = WithPointerChain::read(&process, base).unwrap();
        assert_eq!(result.direct, 999);
        assert_eq!(result.through_ptr, 12345);
    }

    #[test]
    fn read_multi_hop_pointer_chain() {
        #[derive(GameStruct, Debug)]
        struct MultiHop {
            #[offset(0)]
            #[pointer_chain(0, 0)]
            value: u64,
        }

        let target: u64 = 0xBEEF;
        let target_addr: usize = &target as *const u64 as usize;
        let mid_ptr: usize = &target_addr as *const usize as usize;

        // base holds a pointer to mid_ptr, which holds a pointer to target
        let base_data: usize = mid_ptr;
        let process = self_process();
        let base = &base_data as *const usize as usize;
        let result = MultiHop::read(&process, base).unwrap();
        assert_eq!(result.value, 0xBEEF);
    }

    #[test]
    fn read_pointer_chain_with_offsets() {
        #[derive(GameStruct, Debug)]
        struct OffsetChain {
            #[offset(0)]
            #[pointer_chain(8)]
            value: u32,
        }

        // second level: [padding 8 bytes] [target u32]
        let mut level2 = [0u8; 12];
        level2[8..12].copy_from_slice(&7777u32.to_ne_bytes());
        let level2_addr: usize = level2.as_ptr() as usize;

        // base level: pointer to level2
        let process = self_process();
        let base = &level2_addr as *const usize as usize;
        let result = OffsetChain::read(&process, base).unwrap();
        assert_eq!(result.value, 7777);
    }

    #[test]
    fn read_bool_field() {
        #[derive(GameStruct, Debug)]
        struct WithBool {
            #[offset(0)]
            alive: bool,
            #[offset(4)]
            score: u32,
        }

        let mut buf = [0u8; 8];
        buf[0] = 1;
        buf[4..8].copy_from_slice(&100u32.to_ne_bytes());

        let process = self_process();
        let base = buf.as_ptr() as usize;
        let result = WithBool::read(&process, base).unwrap();
        assert!(result.alive);
        assert_eq!(result.score, 100);
    }

    #[test]
    fn read_negative_values() {
        #[derive(GameStruct, Debug)]
        struct Signed {
            #[offset(0)]
            x: i32,
            #[offset(4)]
            y: i32,
        }

        let data: [i32; 2] = [-50, -100];
        let process = self_process();
        let base = data.as_ptr() as usize;
        let result = Signed::read(&process, base).unwrap();
        assert_eq!(result.x, -50);
        assert_eq!(result.y, -100);
    }
}
