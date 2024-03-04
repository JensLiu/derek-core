# Derek Core

This is a personal project for learning Rust and kernel implementation.

We want to firstly create a simple monolithic kernel and then build from there. Maybe add more features and optimisations, or refactor it into a micro-kernel...

# Build
## Prerequisite
Checkout the 
[prerequisite for building XV6](https://pdos.csail.mit.edu/6.828/2020/tools.html), 
it's quite similar.

### Start the kernel
```bash
make qemu
```

### Debug the kernel
```bash
make qemu-gdb
```
And then connect gdb to remote server at port `1234`.

#### Debugging in IDEs
- vscode: checkout the template in `.vscode/launch.json`.
- CLion: checkout this [stackoverflow post](https://stackoverflow.com/questions/55203303/debug-xv6-on-mac-with-clion)


# References
Currently, we've borrowed and modified code and ideas from variaous project and blogs to make it work. We will rewrite some of them in the future.

## Open Source Projects

### Kernels written in C
- [everyone's favourite kernel: XV6](https://github.com/mit-pdos/xv6-public)

### Kernels written in Rust
- [Redox OS](https://gitlab.redox-os.org/redox-os/redox/)
- [core-os-riscv](https://github.com/skyzh/core-os-riscv)
- [rcore-os](https://github.com/rcore-os/rCore)

### Documentations and Blogs
- [Writting an OS in Rust](https://os.phil-opp.com/)
- [The Adventures of OS: Making a RISC-V Operating System using Rust](https://osblog.stephenmarz.com/)
- [The Redox Book](https://doc.redox-os.org/book/)

### Books
- [everyone's favourite OS text book: the XV6 book](https://pdos.csail.mit.edu/6.828/2023/xv6/book-riscv-rev3.pdf)