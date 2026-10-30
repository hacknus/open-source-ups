# Open Source UPS

![pcb_view](renders/ups_main_2026-Oct-30_07-44-32PM-000_CustomizedView568957588_png.png)

UPS are commonly used in server applications to protect against power outages. For home applications, for example to protect a NAS, small Uninterruptible Power Supplies (UPS) do exist, but they all lack the USB HID communication to shut down the NAS/Server/Host when the battery gets low.
For small scale applications like Synology NAS products that operate at 12V, there currently exists no UPS product that is able to buffer the power and communicate over USB when the battery level gets critically low.
Only high voltage solutions (110V or 230V) exist, which also include an inverter and are thus much less efficient and overpowered for a simple 12V Synology NAS.
This is a simple, open source UPS, based around an STM32F405 and written in Rust.

![front_view](renders/ups_case_rev2_2026-Oct-30_07-18-56PM-000_CustomizedView1459810552_png.png)
![back_view](renders/ups_case_rev2_2026-Oct-30_07-19-12PM-000_CustomizedView46267967865_png.png)
![inside_view](renders/ups_case_rev2_2026-Oct-30_07-19-33PM-000_CustomizedView15892773314_png.png)

The USB HID communication is based on the great work done by [abratchik](https://github.com/abratchik) for the Arduino platform [here](https://github.com/abratchik/HIDPowerDevice/tree/master). 
The hardware is essentially just a charging circuit where the remaining capacity is monitored by the MCU.

![pcb_view](renders/ups_case_rev2_2026-Oct-30_07-20-26PM-000_CustomizedView2739970058_png.png)


## Hardware
Attention! This pcb does not contain a battery protection circuit - if you wish to implement one, just use the Keystone 1047 battery case instead of the Keystone 1048P and wire a breakout 2S BMS before connecting the batteries to the PCB.
The hardware can be found in the [hardware folder](hardware), containing step files for the case as well as EAGLE files for the PCB.

## Flashing the firmware
Easiest way to flash: Connect via USB-C to your computer. Press RESET and BOOT button on the back of the PCB, release RST first and then release BOOT. The device is now in DFU mode.

Then run the following:
```sh
cargo build --release
/usr/bin/arm-none-eabi-objcopy -O binary   target/thumbv7em-none-eabihf/release/open-source-ups   firmware.bin
dfu-util -l
dfu-util -d 0483:df11 -a 0 \
  -s 0x08000000:leave -D firmware.bin
```

Alternatively:
Connect the board using a ST-Link V3 (with TagConnect) to a USB port on the computer. Be sure to power the board with an
additional USB-C connector. (ST-Link does not provide power)  
First start the openocd server in a terminal/console window:
```openocd -f interface/stlink-v2-1.cfg -f target/stm32f4x.cfg```   
Flash/Run the code on the hardware using:  
```cargo run --release```  
note: release is important for USB to work reliably!

To set up your system, be sure to
follow [this guide](https://docs.rust-embedded.org/discovery/f3discovery/03-setup/index.html)!
