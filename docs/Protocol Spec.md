# Auxbeam 8-Gang Switch Panel Protocol Specification

This document outlines the reverse-engineered serial interface and protocol used to communicate between the switch panel and the solid-state relay box. This information can be used to build accessory devices which communicate directly with the switch panel assembly.

---

## 1. Physical Layer & Hardware Specification

The interconnection between the Master panel and the Slave relay box utilizes a dedicated 4-conductor cable carrying power, ground, and asymmetric asynchronous data lines (UART).

### 1.1 Cable Pinout (Master Panel Perspective)

| Wire Role | Direction | Description                              |
| :-------- | :-------- | :--------------------------------------- |
| **V-**    | Reference | System Ground / Common reference plane.  |
| **V+**    | Input     | Main DC power supply from the relay box. |
| **TX**    | Output    | Switch Panel TX -> Relay Box RX.         |
| **RX**    | Input     | Relay Box TX -> Switch Panel RX          |

### 1.2 Bus Electrical Characteristics
* **Signaling Type:** Dedicated Transmit (TX) and Receive (RX) lines running asynchronous serial communication (UART).
* **Baud Rate:** 2500 bps.
* **V+ Voltage:** 3.3 V
* **Logic Voltage:** 3.3V
* **Bus Idle State:** Pulled high by Relay Box. Both data lines are pulled high when no active message frame is being transmitted.
* **Driver Configuration:** Line transceivers utilize an **Open-Drain / Open-Collector** layout. Nodes only actively pull the line low to Ground (Logic 0) and float the line for Logic 1.

---

## 2. Frame Architecture

The protocol features dynamic frame lengths optimized for specific data contexts. Every valid frame concludes with an **8-bit Modulo 256 Checksum** calculated as the truncated sum of all preceding bytes.

### 2.1 Standard 9-Byte Frame
Used for runtime control, group creation, and backlight adjustment.

| Byte Offset | Field           | Type | Description                                                |
| :---------- | :-------------- | :--- | :--------------------------------------------------------- |
| `0`         | Sequence ID     | `u8` | Rolling counter.                                           |
| `1`         | Direction       | `u8` | `0x00` = Panel -> Box<br>`0xFF` = Box -> Panel OR Loopback |
| `2`         | Command ID      | `u8` | Establishes packet context (See Section 3).                |
| `3`         | Modifier / Flag | `u8` | Secondary command context or internal status flags.        |
| `4`         | Payload Byte 0  | `u8` | Switch 1 (High Nibble) \| Switch 2 (Low Nibble)            |
| `5`         | Payload Byte 1  | `u8` | Switch 3 (High Nibble) \| Switch 4 (Low Nibble)            |
| `6`         | Payload Byte 2  | `u8` | Switch 5 (High Nibble) \| Switch 6 (Low Nibble)            |
| `7`         | Payload Byte 3  | `u8` | Switch 7 (High Nibble) \| Switch 8 (Low Nibble)            |
| `8`         | Checksum        | `u8` | Sum of Bytes 0 through 7 modulo 256.                       |

### 2.2 Global Parameter 5-Byte Frame
Used for tuning flash/strobe frequency.

| Byte Offset | Field          | Type | Description                                                                                 |
| :---------- | :------------- | :--- | :------------------------------------------------------------------------------------------ |
| `0`         | Sequence ID    | `u8` | Rolling counter (Relay box response echoes truncated lower nibble only; e.g. 0x51 -> 0x01). |
| `1`         | Direction      | `u8` | `0x00` = Panel -> Box<br>`0xFF` = Box -> Panel OR Loopback                                  |
| `2`         | Parameter ID   | `u8` | Target parameter register (e.g., `0x0B` for pulse length).                                  |
| `3`         | Register Value | `u8` | Raw configuration data value.                                                               |
| `4`         | Checksum       | `u8` | Sum of Bytes 0 through 3 modulo 256.                                                        |

---

## 3. Command Registry (`Byte 2`)

The semantic meaning of the payload nibbles shifts completely based on the `Command ID` passed in Byte 2.

| Command ID          | Context Name                   | Processing Mode                   | Wire Target               |
| :------------------ | :----------------------------- | :-------------------------------- | :------------------------ |
| **`0x08` / `0x18`** | Runtime Control                | Mixed Switching Matrix            | Relay Box (Outbound TX)   |
| **`0x07`**          | Master Switch                  | Store/Recall Memory               | Relay Box (Outbound TX)   |
| **`0x0C`**          | Backlight Control              | Backlight Dimming & Color Mapping | Panel Local (Loopback RX) |
| **`0x02` **         | Group Creation and destruction | UI Logic Mapping                  | Panel Local (Loopback RX) |
| 0x0B                | Parameter Set                  | Global Variable Configuration     | Relay Box (Outbound TX)   |

---


## 4. Payload Encoding

### 4.1 Runtime Control Matrix (`0x08` / `0x18`)
Payload bytes 4–7 map left-to-right to physical switches 1–8.
* **Toggle:**
    * `0x0` = **State Low**. Relay is open.
    * `0x1` = **State High**. Relay is closed.
* **Momentary:**
    * `0x2` = **Edge Low Command**. Turn off targeted channel output.
    * `0x3` = **Edge High Command**. Turn on targeted channel output.
* **Pulsed**
	* `0x4` = **Strobe Off.**
	* `0x5` = **Strobe On.**
* **Misc:**
    * `0x8` = **Ignore Mask**. No state change to the channel relay.

### 4.2 Master Switch (`0x07`)
Freezes or restores current state.
* `Byte 3 = 0x00` (Master ON): Turns on outputs in accordance to state saved in cache.
* `Byte 3 = 0x01` (Master OFF): Turns off all outputs and instructs the box to cache the current state in memory.

### 4.3 Switch Panel Backlight Settings (`0x0C`)
Adjusts physical switch panel LED parameters.
* **Byte 3 (Brightness):** 8-bit linear dimmer scale scaling from `0x01` (0.4% minimum brightness) to `0xFF` (100% full brightness).
* **Bytes 4–6 (Color Channels):** Standard 24-bit TrueColor payload mapped linearly as:
    * `Byte 4` = **Red (R)** Intensity ($0 \rightarrow 255$)
    * `Byte 5` = **Green (G)** Intensity ($0 \rightarrow 255$)
    * `Byte 6` = **Blue (B)** Intensity ($0 \rightarrow 255$)
* **Byte 7 (Padding):** Hardcoded to `0x00`.

### 4.4 Global Flash Speed Configuration (`0x0B`)
Modifies the internal flash intervals of the hardware-pulsed strobe configuration.
* **Byte 3 (Pulse Interval Delay):** Value is length measured in milliseconds. 
    * `0x32` (Decimal 50) = Maximum length (Slowest strobe frequency).
    * `0x04` (Decimal 4) = Minimum length (Fastest strobe frequency).

---


## 5. Slave Response & Acknowledgment Patterns

The Slave relay box acts on a strict Query-Response schedule for wirebound commands (`Direction = 0x00`). It utilizes distinct acknowledgment headers and payload echoing to confirm accurate data reception and state execution.

### 5.1 Standard Runtime Acknowledgment (9-Byte)
When the relay box successfully processes a standard runtime command or configuration frame (`Command ID = 0x08`), it returns a 9-byte acknowledgment frame with modified headers:

| Byte Offset | Field Name      | Expected Value | Functional Behavior                                                         |
| :---------- | :-------------- | :------------- | :-------------------------------------------------------------------------- |
| `0`         | Sequence ID     | `Varies`       | Direct echo of the rolling Sequence ID from the Query frame.                |
| `1`         | Direction       | `0xFF`         | Outbound from Relay Box to Switch Panel.                                    |
| `2`         | Ack Command ID  | `0x18`         | Shifts high nibble by 0x1 to indicate response.                             |
| `3`         | Status Modifier | `0x00`         | Drops from `0x08` to `0x00` to indicate nominal execution with zero errors. |
| `4–7`       | Payload Echo    | `Varies`       | Byte-for-byte echo of the received 4-byte switch matrix.                    |
| `8`         | Checksum        | `Varies`       | Sum of Bytes 0 through 7 modulo 256.                                        |

**Example:**
* Query (Panel):    `25 00 08 08 38 88 88 88 05`
* Response (Box): `25 FF 18 00 38 88 88 88 0C`

---

### 5.2 Global Parameter Sync Acknowledgment (5-Byte)
When tuning fine-grained register parameters like strobe speed (`Parameter ID = 0x0B`), the relay box response truncates the upper nibble, setting it to `0x0`:

| Byte Offset | Field Name       | Expected Value | Functional Behavior                                                                              |
| :---------- | :--------------- | :------------- | :----------------------------------------------------------------------------------------------- |
| `0`         | Sequence ID      | `Lower Nibble` | The box strips the upper nibble of the Sequence ID (e.g., Query `0x52` becomes Response `0x02`). |
| `1`         | Direction        | `0xFF`         | Outbound from Relay Box to Switch Panel.                                                         |
| `2`         | Ack Parameter ID | `0x1B`         | Shifts high nibble by 0x1 to indicate response.                                                  |
| `3`         | Register Value   | `Varies`       | Direct echo of the written configuration value (e.g., `0x32`) to verify data integrity.          |
| `4`         | Checksum         | `Varies`       | Sum of Bytes 0 through 3 modulo 256.                                                             |

**Example:**
* Query (Panel): `52 00 0B 32 8F`
* Response (Box): `02 FF 1B 32 4E`

---

### 5.3 Local Local-Loopback Commands (Null Response Pattern)
Some commands, like backlight control command are generated for the master switch panel by the master switch panel. These are sent to "loopback," where they are sent on the RX line by the panel and addressed to the panel. This is interpreted to be an acknowledgement of the internal change. These commands can be sent by another device on that line to change those settings.