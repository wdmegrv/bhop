// Linux Bunnyhop hack for Counter Strike: Source
// Copyright (C) 2022 Wadim Egorov <wdmegrv@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

/* Prevent a spurious 'unused_imports' warning */
#[macro_use]

extern crate ctor;

use libc::c_char;
use libc::c_void;
use libc::dl_phdr_info;
use libc::size_t;
use libc::dl_iterate_phdr;

use std::ffi::CStr;
use std::ffi::CString;
use std::mem::transmute;
use std::str::FromStr;
use std::time::Duration;
use std::thread;
use std::thread::sleep;
use std::ptr;
use std::slice::from_raw_parts;

/* Some nice external libraries */
use detour::static_detour;
use inputbot::KeybdKey::SpaceKey;
use libloading::Library;
use memscan::Pattern;
use memscan::find_pattern;


/* Globals for BHOP */
static mut CLIENT: *const u8 = ptr::null_mut();
static mut DO_JUMP: *mut u32 = ptr::null_mut();
static mut BHOP_ENABLED: bool = false;

/* Hook definitions for landing/leaving the ground */
static_detour! {
    static hook_on_ground_leave: unsafe extern fn(i32, *const u32);
    static hook_on_ground_land: unsafe extern fn(*const i32, *const i32) -> *const u32;
}

/* Actual game hooks */
fn on_ground_leave_detour(p1: i32, p2: *const u32) {
    unsafe {
        hook_on_ground_leave.call(p1, p2);
        if BHOP_ENABLED {
            *DO_JUMP = 4;
        }
    }
}

fn on_ground_land_detour(p1: *const i32, p2: *const i32) -> *const u32 {
    unsafe {
        if BHOP_ENABLED {
            *DO_JUMP = 5;
        }

        return hook_on_ground_land.call(p1, p2);
    }
}

/* Print to the internal game console (allocated with 'hl2_linux -console') */
fn css_console(msg: &str) {
    unsafe {
        /* Typedef ConMsg(char const*, ...) from libtier0.so */
        let libtier0: Library = libloading::Library::new("libtier0.so").unwrap();
        let conmsg: libloading::Symbol<unsafe extern fn(*const c_char)> =
            libtier0.get(b"_Z6ConMsgPKcz").unwrap();

        let c_msg = CString::new(msg).unwrap();
        let c_msg = c_msg.as_ptr() as *const c_char;
        conmsg(c_msg);
    }
}

/* Find game stuff in hl2_linux */
fn find_bhop_locations() {

    /* Patterns for required game functions/pointers. See README.md on how to find functions */
    let sig_do_jump = match Pattern::from_str("8B 3D ? ? ? ? 89 DA 83 CA 02 F7 C7 03 00 00 00 0F 45 DA") {
        Ok(pattern) => pattern,
        Err(idx) => { println!("Pattern parse error at {}", idx); return; },
    };

    let sig_leave_ground = match Pattern::from_str("55 89 e5 56 53 83 ec 10 8b 5d 0c 8b 75 08 8b 0d ? ? ? ? 8b 13 83 fa ff") {
        Ok(pattern) => pattern,
        Err(idx) => { println!("Pattern parse error at {}", idx); return; },
    };

    let sig_on_ground_land = match Pattern::from_str("55 89 e5 57 56 53 31 db 83 ec 2c 8b 55 0c") {
        Ok(pattern) => pattern,
        Err(idx) => { println!("Pattern parse error at {}", idx); return; },
    };

    /* Don't care about the client module size :O */
    let mem_client: &[u8] = unsafe { from_raw_parts(CLIENT, 0xA00000) };

    /* Scan for patterns in own process memory */
    let do_jump_scan = find_pattern(mem_client, sig_do_jump);
    if do_jump_scan.is_empty() {
        css_console("sig_do_jump not found\n");
        return;
    }

    let leave_ground_scan = find_pattern(mem_client, sig_leave_ground);
    if leave_ground_scan.is_empty() {
        css_console("leave_ground_scan not found\n");
        return;
    }

    let on_ground_land_scan = find_pattern(mem_client, sig_on_ground_land);
    if on_ground_land_scan.is_empty() {
        css_console("on_ground_land_scan not found\n");
        return;
    }

    unsafe {
        /* Extract DO_JUMP pointer from instruction */
        let mem_do_jump: *const u32 = do_jump_scan[0].as_ptr().add(2) as * const u32;
        DO_JUMP = *mem_do_jump as *mut u32;

        css_console(format!("do_jump_scan @ {:p}\n", do_jump_scan[0]).as_str());
        css_console(format!("leave_ground_scan @ {:p}\n", leave_ground_scan[0]).as_str());
        css_console(format!("on_ground_land_scan @ {:p}\n", on_ground_land_scan[0]).as_str());
        css_console(format!("DO_JUMP @ {:p}\n", DO_JUMP).as_str());

        /* Initialize game function hooks */
        let target: unsafe extern fn(i32, *const u32) = transmute(leave_ground_scan[0].as_ptr());
        hook_on_ground_leave.initialize(target, on_ground_leave_detour).unwrap().enable().unwrap();

        let target: unsafe extern fn(*const i32, *const i32) -> *const u32 = transmute(on_ground_land_scan[0].as_ptr());
        hook_on_ground_land.initialize(target, on_ground_land_detour).unwrap().enable().unwrap();
    }
}

unsafe extern fn dl_it_callback(info: *mut dl_phdr_info, _size: size_t, _data: *mut c_void) -> i32 {
    let name: String = CStr::from_ptr((*info).dlpi_name).to_string_lossy().into_owned();

    /* Found loaded client.so, now let's search for required game functions */
    if name.contains("/client.so") {
        css_console(format!("{} @ {:#0x}\n", name, (*info).dlpi_addr).as_str());
        CLIENT = (*info).dlpi_addr as *const u8;
        find_bhop_locations();
    }

    return 0;
}

/* Make sure our shared object init() gets called when loaded by dlopen() */
#[ctor]
fn init() {

    /* Run in own thread & wait a bit before we start initializing our own stuff */
    thread::spawn(|| {

        sleep(Duration::from_secs(10));
        unsafe {
            /* Use dl_iterate_phdr() to get module base addresses */
            let data: *mut c_void = ptr::null_mut();
            dl_iterate_phdr(Some(dl_it_callback), data);
        }

        /* Enable BHOP when Space is pressed */
        SpaceKey.bind(|| {
            loop {
                unsafe {
                    BHOP_ENABLED = SpaceKey.is_pressed();
                }
                sleep(Duration::from_millis(20));
            }
        });

        css_console("BHOP initialized\n");
        inputbot::handle_input_events();
    });
}
