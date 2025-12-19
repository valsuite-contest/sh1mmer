#!/bin/busybox sh

# Hello World SH1MMER Payload
# This is a minimal example showing how to create a custom message shim

set -eE

# Arguments passed from init script
STATEFUL_MNT="$1"
STATEFUL_DEV="$2"
BOOTSTRAP_DEV="$3"
ARCHITECTURE="${4:-unknown}"

# Color codes for pretty output
COLOR_RESET="\033[0m"
COLOR_GREEN="\033[1;32m"
COLOR_CYAN="\033[1;36m"
COLOR_YELLOW="\033[1;33m"
COLOR_MAGENTA="\033[1;35m"

# Clear screen and hide cursor
printf "\033[2J\033[H\033[?25l"

# Display fancy hello world message
printf "${COLOR_CYAN}"
echo "╔════════════════════════════════════════════════════════════╗"
echo "║                                                            ║"
echo "║              Hello World Custom SH1MMER!                  ║"
echo "║                                                            ║"
echo "╚════════════════════════════════════════════════════════════╝"
printf "${COLOR_RESET}\n"

printf "${COLOR_GREEN}Congratulations!${COLOR_RESET} You have successfully created a custom SH1MMER payload.\n\n"

# Show system information
printf "${COLOR_YELLOW}System Information:${COLOR_RESET}\n"
echo "  Architecture:      $ARCHITECTURE"
echo "  Kernel:            $(uname -r)"
echo "  Stateful Device:   $STATEFUL_DEV"
echo "  Bootstrap Device:  $BOOTSTRAP_DEV"
echo ""

printf "${COLOR_YELLOW}Hardware:${COLOR_RESET}\n"
echo "  CPU:               $(grep "model name" /proc/cpuinfo | head -1 | cut -d: -f2 | xargs)"
echo "  Memory:            $(free -h | awk '/^Mem:/ {print $2}')"
echo ""

# Try to find ChromeOS root filesystem
printf "${COLOR_YELLOW}Attempting to identify ChromeOS installation...${COLOR_RESET}\n"
ROOTFS_DEV=""
bootstrap_num="${BOOTSTRAP_DEV##*[!0-9]}"
ROOTFS_DEV="${BOOTSTRAP_DEV%${bootstrap_num}}$((bootstrap_num + 1))"

if [ -b "$ROOTFS_DEV" ]; then
    echo "  Root Device:       $ROOTFS_DEV"
    
    # Try to mount and read version
    ROOTFS_MNT=$(mktemp -d)
    if mount -o ro "$ROOTFS_DEV" "$ROOTFS_MNT" 2>/dev/null; then
        if [ -f "$ROOTFS_MNT/etc/lsb-release" ]; then
            VERSION=$(grep "^CHROMEOS_RELEASE_VERSION=" "$ROOTFS_MNT/etc/lsb-release" | cut -d= -f2)
            BOARD=$(grep "^CHROMEOS_RELEASE_BOARD=" "$ROOTFS_MNT/etc/lsb-release" | cut -d= -f2)
            echo "  ChromeOS Version:  $VERSION"
            echo "  Board:             $BOARD"
        fi
        if ! umount "$ROOTFS_MNT" 2>/dev/null; then
            echo "  Warning: Failed to unmount $ROOTFS_MNT" >&2
        fi
    fi
    rmdir "$ROOTFS_MNT" 2>/dev/null || :
else
    echo "  Root Device:       Not found"
fi

echo ""
printf "${COLOR_MAGENTA}════════════════════════════════════════════════════════════${COLOR_RESET}\n"
echo ""
echo "This is a minimal example payload. In a real SH1MMER shim, you would:"
echo "  • Copy the ChromeOS root filesystem to tmpfs"
echo "  • Patch system files to bypass enrollment"
echo "  • Load additional tools and payloads"
echo "  • Present a menu with various options"
echo ""
echo "For a full implementation, see the 'sh1mmer_bw' or 'sh1mmer_legacy' payloads."
echo ""

# Show available block devices
printf "${COLOR_YELLOW}Available Storage Devices:${COLOR_RESET}\n"
lsblk -d -o NAME,SIZE,TYPE --noheadings 2>/dev/null | awk '{print "  /dev/" $1 " (" $2 ")"}' || echo "  Unable to list devices"
echo ""

# Simple menu
printf "${COLOR_CYAN}What would you like to do?${COLOR_RESET}\n"
echo "  1) Drop to a shell (busybox)"
echo "  2) Show full partition layout"
echo "  3) Reboot"
echo ""

# Show cursor and enable input
printf "\033[?25h"
stty echo 2>/dev/null || :

while true; do
    printf "Enter choice [1-3]: "
    read -r choice
    
    case "$choice" in
        1)
            echo ""
            printf "${COLOR_GREEN}Entering shell...${COLOR_RESET}\n"
            echo "Type 'exit' to return to the menu."
            echo ""
            export PATH=/bin:/usr/bin:/sbin:/usr/sbin
            sh || :
            printf "\n${COLOR_CYAN}What would you like to do?${COLOR_RESET}\n"
            echo "  1) Drop to a shell (busybox)"
            echo "  2) Show full partition layout"
            echo "  3) Reboot"
            echo ""
            ;;
        2)
            echo ""
            printf "${COLOR_GREEN}Partition Layout:${COLOR_RESET}\n"
            lsblk -o NAME,SIZE,TYPE,MOUNTPOINT --noheadings 2>/dev/null || echo "Unable to read partition table"
            echo ""
            echo "Press Enter to continue..."
            read -r
            printf "${COLOR_CYAN}What would you like to do?${COLOR_RESET}\n"
            echo "  1) Drop to a shell (busybox)"
            echo "  2) Show full partition layout"
            echo "  3) Reboot"
            echo ""
            ;;
        3)
            echo ""
            printf "${COLOR_YELLOW}Rebooting in 3 seconds...${COLOR_RESET}\n"
            sleep 3
            reboot
            # Wait forever in case reboot fails
            tail -f /dev/null
            ;;
        *)
            printf "${COLOR_YELLOW}Invalid choice. Please enter 1, 2, or 3.${COLOR_RESET}\n"
            ;;
    esac
done
