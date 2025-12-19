# SH1MMER Documentation Summary

This document provides a quick navigation guide to all SH1MMER documentation.

## Quick Links

### For End Users
- **[Main README](README.md)** - Start here for basic usage instructions
- **[Building a Shim](README.md#building-a-beautiful-world-shim)** - How to build and flash a SH1MMER image
- **[Booting](README.md#booting-into-a-shim)** - How to boot into a SH1MMER shim
- **[Patches & Workarounds](README.md#patch-information-and-workarounds)** - Known patches and how to work around them

### For Developers & Researchers
- **[EXPLOIT_FLOW.md](EXPLOIT_FLOW.md)** - Comprehensive technical documentation
  - Detailed explanation of how the exploit works
  - The complete build and boot process
  - Creating custom payloads from scratch
  
- **[Custom Payload Examples](examples/)** - Hands-on examples
  - [Hello World Example](examples/hello_world/) - Minimal working payload
  - [Examples Guide](examples/README.md) - Structure, best practices, use cases

### For Payload Developers
- **[wax/readme.br0ker.md](wax/readme.br0ker.md)** - Building shims with Br0ker update payloads
- **[BUILDING_FROM_SCRATCH.md](BUILDING_FROM_SCRATCH.md)** - Build minimal shims without wax (advanced)
- **Payload Implementations**:
  - `wax/sh1mmer_bw/` - Beautiful World GUI payload (recommended)
  - `wax/sh1mmer_legacy/` - Legacy CLI payload

## Documentation Overview

### EXPLOIT_FLOW.md (488 lines)

**Comprehensive technical guide covering:**

1. **How the Exploit Works**
   - The vulnerability in RMA shims
   - Attack surface and key insights
   - Why only KERNEL partitions are verified

2. **The Build Process**
   - How wax.sh works
   - Partition layout transformation
   - Build command examples

3. **The Boot Process**
   - Stage-by-stage boot flow
   - From firmware to final payload
   - Visual diagrams included

4. **Creating Custom Shims**
   - Step-by-step tutorial
   - Complete "Hello World" example
   - Understanding payload structure
   - What you can and cannot do

### BUILDING_FROM_SCRATCH.md (NEW - 16,772 characters)

**Deep dive for advanced users who want to build without wax:**

1. **Understanding RMA Shim Structure**
   - Original partition layout
   - Why the exploit works
   - What can be modified

2. **Core Concepts**
   - The read-only bit (0x464 offset)
   - Partition table manipulation with cgpt/sfdisk
   - Loop device setup

3. **The Critical Exploit Mechanism**
   - How boot hijack works
   - Two-stage boot process
   - lsb-factory file importance

4. **Step-by-Step Build Process**
   - Enable RW access on partitions
   - Shrink ROOT-A to minimum
   - Create bootloader partition
   - Create SH1MMER partition
   - Partition swapping technique

5. **Minimal Build Script**
   - Complete standalone script (no wax dependencies)
   - Under 100 lines
   - Creates ~90MB shim (vs 1GB+ with wax)

6. **Advanced Optimization**
   - Ultra-minimal approaches
   - Size calculations
   - Understanding what can/cannot be modified

### examples/README.md (331 lines)

**Practical guide for creating payloads:**

- Available examples and what they demonstrate
- Prerequisites and quick start guide
- Detailed payload structure explanation
- Creating your own payload from scratch
- Common use cases (diagnostic, automation, educational)
- Advanced topics (binaries, graphics, persistence)
- Debugging techniques
- Best practices and limitations

### examples/hello_world/README.md (193 lines)

**Minimal working example:**

- What the example does
- Directory structure
- How to build and use
- Understanding the code
- Customization ideas
- Troubleshooting tips
- Security considerations

## Learning Path

### Beginner Path
1. Read [Main README](README.md) - Understand what SH1MMER is
2. Follow [Building a Shim](README.md#building-a-beautiful-world-shim) - Create your first shim
3. Read [EXPLOIT_FLOW.md - How it Works](EXPLOIT_FLOW.md#how-the-exploit-works) - Understand the basics

### Developer Path
1. Read [EXPLOIT_FLOW.md](EXPLOIT_FLOW.md) - Full technical understanding
2. Review [examples/hello_world](examples/hello_world/) - See a minimal example
3. Study `wax/sh1mmer_bw/` or `wax/sh1mmer_legacy/` - Full implementations
4. Create your own payload using [examples/README.md](examples/README.md)

### Researcher Path
1. Read [EXPLOIT_FLOW.md](EXPLOIT_FLOW.md) - Technical deep dive
2. Study the boot process diagrams
3. Analyze `wax/wax.sh` - Build system internals
4. Review `wax/bootstrap/` and `wax/sh1mmer_*/bootstrap/` - Init process
5. Explore patches and workarounds in [Main README](README.md#patch-information-and-workarounds)

## Key Concepts

### Partition Layout
```
Before:  p1: STATE    p2: KERNEL-A (signed)   p3: ROOT-A (large)
After:   p1: SH1MMER  p2: KERNEL-A (signed)   p3: ROOT-A (shrunk)  p4: Bootloader
```

### Boot Flow
```
Firmware → Kernel (signed) → Bootstrap Init (p4) → SH1MMER Init (p1) → 
Patch ROOT → Switch Root → Payload (Beautiful World / Legacy)
```

### Payload Structure
```
payload/
├── init                    # Entry point
├── bootstrap/              # Early boot scripts
│   └── noarch/
└── root/                   # Files overlaid on ChromeOS
    └── noarch/
```

## File Size Reference

| File | Lines | Purpose |
|------|-------|---------|
| EXPLOIT_FLOW.md | 488 | Complete technical documentation |
| examples/README.md | 331 | Payload development guide |
| examples/hello_world/README.md | 193 | Example tutorial |
| examples/hello_world/init | 38 | Example entry point |
| examples/hello_world/bootstrap/noarch/hello_world.sh | 139 | Example payload |

**Total documentation:** Over 1,000 lines of comprehensive guides and examples

## External Resources

- **Blog Post**: https://blog.coolelectronics.me/breaking-cros-2/
- **GitHub**: https://github.com/MercuryWorkshop/sh1mmer
- **Related Projects**:
  - [shimboot](https://github.com/ading2210/shimboot) - Boot Linux from RMA shim
  - [CryptoSmite](https://github.com/FWNavy/CryptoSmite) - Enrollment bypass
  - [Br0ker](wax/readme.br0ker.md) - Downgrade and bypass

## Frequently Asked Questions

**Q: Where do I start?**  
A: Read the [Main README](README.md) first, then follow the building instructions.

**Q: I want to understand how it works technically?**  
A: Read [EXPLOIT_FLOW.md](EXPLOIT_FLOW.md) for a complete explanation.

**Q: How do I create a custom payload?**  
A: Start with the [Hello World Example](examples/hello_world/), then read [examples/README.md](examples/README.md).

**Q: Can I modify the kernel?**  
A: No, the KERNEL partition is signed and verified by the firmware.

**Q: Will changes persist after reboot?**  
A: No, SH1MMER runs in RAM (tmpfs). Changes are temporary unless you modify the stateful partition.

**Q: What boards are supported?**  
A: See the board list in the [Main README](README.md). You need a raw RMA shim for your specific board.

**Q: Is this legal?**  
A: This is for educational and research purposes. Only use on devices you own or have authorization to modify.

## Contributing

Found an error in the documentation? Want to add an example?

1. Check existing documentation first
2. Follow the structure and style of existing docs
3. Test any code examples on real hardware
4. Submit a pull request with clear description

## Credits

Documentation created as part of the SH1MMER project by Mercury Workshop:
- CoolElectronics - Original exploit
- r58Playz - GUI and initial scripts
- Sharp_Jack - wax build system
- OlyB - Current maintainer
- And many other contributors

## License

This documentation follows the same license as the main SH1MMER project.

---

**Remember:** This is for educational and research purposes only. Use responsibly and ethically.

Last updated: December 2024
