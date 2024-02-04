# A Linker Bug in MacOS

> TLDR, the MacOS linker has an assertion failure when trying to handle the relocation of global data.

I wasn't really the one who originally found this. That credit goes to [Luna Razzaghipour](https://github.com/lunacookies).

This was found while trying to compile the [capy](https://github.com/capy) programming language on an M2 mac.

I'm really not sure that there's a problem with the object file here. It's important to note that not only does LLVM's `lld` handle the object file correctly, `ld` itself works correctly if you pass `-ld-classic`. This issue only comes up with `-ld-new`.

Also the fact that this is an assertion failure as opposed to an error makes me believe this is a bug.

This project is written in Rust to reproduce the exact conditions the bug was found in. If you don't have rust installed I included the object file produced by my M2 macbook air. If you'd rather generate the file yourself you can install Rust and run the following command,

```shell
cargo run
```

This will generate the object file.

The program can also run `ld` itself. To run it with `-ld-new`,

```shell
cargo run -- with_bug
```

or with `-ld-classic`,

```shell
cargo run -- without_bug
```

Here is the exact `ld` command that produces the bug,

```shell
ld -platform_version macos 14.2 14.2 -syslibroot /Library/Developer/CommandLineTools/SDKs/MacOSX.sdk -lSystem -ld_new -o my_awesome_program  my_awesome_program.o
```

Replacing `-ld-new` with `-ld-classic` will successfully produce an executable.

## More Technical Details

I don't know enough about the internals of `ld` to really know the exact cause of this bug. What I could figure out is that `-ld-new` only crashes when global data (in this case, "Hello, World\0") is accessed. The backtrace from `ld` also starts in a function named `addFixupFromRelocations`, which confirms that this is a relocation issue.

Here is the exact panic I got on my machine,

```
0  0x104ce6f2c  __assert_rtn + 72
1  0x104c2dec4  ld::InputFiles::SliceParser::parseObjectFile(mach_o::Header const*) const + 22976
2  0x104c3a404  ld::InputFiles::parseAllFiles(void (ld::AtomFile const*) block_pointer)::$_7::operator()(unsigned long, ld::FileInfo const&) const + 420
3  0x188f34950  _dispatch_client_callout2 + 20
4  0x188f491a4  _dispatch_apply_invoke_and_wait + 176
5  0x188f48464  _dispatch_apply_with_attr_f + 1176
6  0x188f48650  dispatch_apply + 96
7  0x104cb81e0  ld::AtomFileConsolidator::parseFiles(bool) + 292
8  0x104c56b08  main + 9252
ld: Assertion failed: (pattern[0].addrMode == addr_other), function addFixupFromRelocations, file Relocations.cpp, line 700.
```
