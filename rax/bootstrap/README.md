# Bootstrap Directory

This directory contains the bootloader files needed for SH1MMER shim images.

## Structure

```
bootstrap/
├── noarch/              # Architecture-independent files
│   ├── sbin/
│   │   └── init         # Initial boot script (uses busybox)
│   └── usr/sbin/
│       └── factory_bootstrap.sh  # Factory bootstrap helper
├── x86_64/              # x86_64-specific files
│   └── bootstrap/
│       └── busybox      # Busybox binary for x86_64
└── aarch64/             # aarch64-specific files
    └── bootstrap/
        └── busybox      # Busybox binary for aarch64
```

## What is Busybox?

Busybox is a single binary that provides many common Unix utilities in a compact form.
It's used as the bootloader because it provides all the necessary tools (sh, mount, tar, etc.)
in a single small binary that works in the recovery environment.

## Using Custom Bootstrap

You can use a custom bootstrap directory by specifying `--bootloader-dir`:

```bash
rax -i shim.bin --bootloader-dir /path/to/custom/bootstrap --payload-dir /path/to/payload
```

Your custom bootstrap directory should follow the same structure as shown above.
