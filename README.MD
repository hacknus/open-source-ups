### Open Source UPS

Connect the board using a ST-Link V3 (with TagConnect) to a USB port on the computer. Be sure to power the board with an
additional USB-C connector. (ST-Link does not provide power)  
First start the openocd server in a terminal/console window:
```openocd -f interface/stlink-v2-1.cfg -f target/stm32f4x.cfg```   
Flash/Run the code on the hardware using:  
```cargo run --release```  
note: release is important for USB to work reliably!

To set up your system, be sure to
follow [this guide](https://docs.rust-embedded.org/discovery/f3discovery/03-setup/index.html)!