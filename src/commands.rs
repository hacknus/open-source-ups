pub fn process_ups_command(buffer: &[u8]) -> &str {
    if buffer.is_empty() {
        return "ERR"; // Return an error for empty buffer
    }

    match buffer[0] {
        b'^' => match buffer.get(1) {
            Some(&b'A') => "SMART-UPS 700",        // Model string
            Some(&b'N') => "ACK; ON",              // Turn on UPS (dummy response)
            Some(&b'Z') => "CAPABILITIES_STRING",  // Capability string
            _ => "UNKNOWN COMMAND",               // Unknown sub-command after '^'
        },
        b'A' => "OK; Light show started",         // Front panel test
        b'B' => "27.87",                          // Battery voltage
        b'C' => "036.0",                          // Internal temperature
        b'D' => "!, then $",                      // Runtime calibration
        b'E' => "336",                            // Automatic self-test intervals
        b'F' => "60.00",                          // Line frequency, Hz
        b'G' => "UNKNOWN",                        // Cause of transfer
        b'K' => "OK",                             // Shutdown with grace period
        b'L' => "118.3",                          // Input line voltage
        b'M' => "118.9",                          // Maximum line voltage
        b'N' => "118.9",                          // Minimum line voltage
        b'O' => "118.3",                          // Output voltage
        b'P' => "023.5",                          // Power load %
        b'Q' => "08",                             // Status flags
        b'R' => "BYE",                            // Turn dumb
        b'S' => "OK",                             // Soft shutdown
        b'U' => "!, then $",                      // Simulate power failure
        b'V' => "GWD",                            // Firmware revision
        b'W' => "OK",                             // Self-test
        b'X' => "OK",                             // Self-test results
        b'Y' => "SM",                             // Enter smart mode
        b'Z' => "n/a",                            // Shutdown immediately
        b'a' => "PROTOCOL_INFO",                  // Protocol info
        b'b' => "50.9.D",                         // Firmware revision
        b'c' => "UPS_IDEN",                       // UPS local id
        b'e' => "00",                             // Return threshold
        b'f' => "099.0",                          // Battery level
        b'g' => "024",                            // Nominal battery voltage
        b'h' => "042.4",                          // Measure-UPS: Ambient humidity
        b'i' => "00",                             // Measure-UPS: Dry contacts
        b'j' => "0327",                           // Estimated runtime
        b'k' => "0",                              // Alarm delay
        b'l' => "103",                            // Low transfer voltage
        b'm' => "11/29/96",                       // Manufacturing date
        b'n' => "WS9643050926",                   // Serial number
        b'o' => "115",                            // On-battery voltage
        b'p' => "020",                            // Shutdown grace delay
        b'q' => "02",                             // Low battery warning
        b'r' => "000",                            // Wakeup delay
        b's' => "H",                              // Sensitivity
        b'u' => "132",                            // Upper transfer voltage
        b'v' => "4Kx",                            // Measure-UPS: Firmware
        b't' => "80.5",                           // Measure-UPS: Ambient temperature
        b'x' => "11/29/96",                       // Last battery change
        b'y' => "© APCC",                         // Copyright notice
        b'z' => "CLEAR",                          // Reset to factory settings
        b'@' => "OK",                             // Shutdown and return
        b'~' => "n/a",                            // Register #1
        b'/' => "n/a",                            // Load current
        b'\\' => "n/a",                           // Apparent load power
        _ => "UNKNOWN COMMAND",                   // Unknown command
    }
}