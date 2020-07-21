target := "riscv64imac-unknown-none-elf"
mode := "debug"
build-path := "target/" + target + "/" + mode + "/"
kernel_file := build-path + "pcrel-os-lab"
bin_file := build-path + "kernel.bin"

objdump := "riscv64-unknown-elf-objdump"
objcopy := "rust-objcopy --binary-architecture=riscv64"

build: kernel
    @{{objcopy}} {{kernel_file}} --strip-all -O binary {{bin_file}}

kernel:
    @cargo build --target={{target}}

asm: build
    @{{objdump}} -D {{kernel_file}} | less

qemu: build
    @qemu-system-riscv64 \
            -machine virt \
            -nographic \
            -bios default \
            -device loader,file={{bin_file}},addr=0x80200000

debug: build
    @qemu-system-riscv64 \
            -machine virt \
            -nographic \
            -bios default \
            -device loader,file={{bin_file}},addr=0x80200000 \
            -gdb tcp::1234 -S

gdb: 
    @gdb --eval-command="file {{kernel_file}}" \
        --eval-command="target remote localhost:1234"
