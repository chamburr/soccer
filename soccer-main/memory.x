MEMORY
{
    /* NOTE 1 K = 1 KiBi = 1024 bytes */
    BOOT2                             : ORIGIN = 0x10000000, LENGTH = 0x100
    BOOTLOADER_STATE                  : ORIGIN = 0x10006000, LENGTH = 4K
    FLASH                             : ORIGIN = 0x10007000, LENGTH = 512K
    DFU                               : ORIGIN = 0x10087000, LENGTH = 516K
    RAM                               : ORIGIN = 0x20000000, LENGTH = 264K
}

__bootloader_state_start = ORIGIN(BOOTLOADER_STATE) - ORIGIN(BOOT2);
__bootloader_state_end = ORIGIN(BOOTLOADER_STATE) + LENGTH(BOOTLOADER_STATE) - ORIGIN(BOOT2);

__bootloader_dfu_start = ORIGIN(DFU) - ORIGIN(BOOT2);
__bootloader_dfu_end = ORIGIN(DFU) + LENGTH(DFU) - ORIGIN(BOOT2);
