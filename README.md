Linux Bunnyhop hack for Counter Strike: Source
==============================================

A Bunnyhop hack for Counter Strike: Source written in Rust.
Nothing special new about anything in terms of game hacking, but
more of a project for me to get a first feeling for the Rust workflow.


## Usage

Inject the generated `libbhop.so` shared library into the `hl2_linux` process
using your favorite injection method, e.g. use of `LD_PRELOAD`:

Most simple way is to add an export inside Steams boostrap `hl2.sh`.

```
~/.steam/debian-installation/steamapps/common/Counter-Strike Source:
export LD_PRELOAD=/path/to/libbhop.so
```

After you started Counter Strike, you should see the following output on the
internal game console:

```
client.so @ 0xcb666000
do_jump_scan @ 0xcbd079d6
leave_ground_scan @ 0xcba72360
on_ground_land_scan @ 0xcba72220
DO_JUMP @ 0xcc2544e8
BHOP initialized
```

At this point the Bunnyhop hack is initialized an ready.
Keep the `SPACE` key pressed to see the effects.


## Building with Cargo

As the `hl2_linux` binary and it's libraries are compiled as ELF 32-bit files
for the i686 architecture, we need to provide our own library in the same
format. So if you are on a x86_64 system, you will need to cross compile the
library.

* Setup a i686 build environment
  See also: https://rust-lang.github.io/rustup/cross-compilation.html
  ```
  rustup target add i686-unknown-linux-musl
  ```
* Install all dependencies, for Ubuntu
  ```
  sudo dpkg --add-architecture i386
  sudo apt-get update
  sudo apt-get install \
    libc6-dev:i386 \
    gcc:i386 \
    libinput-dev:i386 \
    libx11-dev:i386 \
    libxtst-dev:i386 \
    musl-tools
    # And maybe more libraries are required
    # I don't have the full install log anymore :(
  ```
* After that you should be able to run a cargo build
  ```
  export PKG_CONFIG_SYSROOT_DIR="/usr/lib/i386-linux-gnu/"
  RUSTFLAGS="-C target-feature=-crt-static" cargo build --target i686-unknown-linux-musl
  ```
* Don't blame me if it's not compiling


## Finding needed game functions

Since patterns tend to break over time it is helpful to know on how to
find outdated game functions again.

Tools used: GDB & your favorite memory scanning tool.


### Jump Pointer

Used to execute the ingame jump. Known memory values are
`DO_JUMP=4` and `DO_JUMP=5 (jump)`. Basically, you just have to use your memory
scanning tool and scan for 4 or 5 depending if you are jumping or not.

After we found the memory address, we need to find a user (code) inside the
process that is pointing to this memory address. Simply go with GDB and
set a watchpoint on the content of this memory address, e.g.: `watch *<addr>`.
A jump should trigger the GDB watchpoint and points you to some (code) users.
From here on you can start and rebuild your new pattern.


### Hooks for leaving and landing on ground

There is a memory address that comes with the values 0 or 1 which tells
us if we are currently on the ground or not (`ON_GROUND=1/0`).
Find the location and use the same watchpoint method as described above.
GDB triggers twice on this location: After jumping/leaving the ground and
if you come back again. Now you know your hook locations.
