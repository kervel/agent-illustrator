# Data Model: MOSFET Driver Example

## Component Templates

All templates are defined **in-file** within `mosfet-driver.ail` using AIL's template syntax.

---

### Template: Resistor

**Purpose**: Represents a resistor with configurable value label

**Parameters**:
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `value` | string | "R" | Resistance value (e.g., "10kΩ") |

**Visual**: Horizontal rectangle with value label (simplified schematic style)

**Anchors**:
| Anchor | Position | Direction | Purpose |
|--------|----------|-----------|---------|
| `left` | Body left edge | left | Input connection |
| `right` | Body right edge | right | Output connection |

**Sketch**:
```
  ┌─────────┐
──┤  10kΩ   ├──
  └─────────┘
 left      right
```

---

### Template: N-Channel MOSFET

**Purpose**: Represents an N-channel enhancement MOSFET

**Parameters**: None (standard symbol)

**Visual**: Simplified MOSFET symbol with G/D/S labels

**Anchors**:
| Anchor | Position | Direction | Purpose |
|--------|----------|-----------|---------|
| `gate` | Left side | left | Gate input |
| `drain` | Top | up | Drain (high-side) |
| `source` | Bottom | down | Source (low-side) |

**Sketch**:
```
        drain
          │
    ──┬───┤
  gate │   │
    ──┴───┤
          │
        source
```

---

### Template: LED

**Purpose**: Represents an LED with configurable color

**Parameters**:
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `color` | color | accent-1 | LED color for fill |

**Visual**: Triangle (diode) with emission indicator

**Anchors**:
| Anchor | Position | Direction | Purpose |
|--------|----------|-----------|---------|
| `anode` | Top | up | Positive connection (+) |
| `cathode` | Bottom | down | Negative connection (-) |

**Sketch**:
```
      anode
        │
        ▼
      ─────
       \ /
        V    )) (emission)
      ─────
        │
      cathode
```

---

### Template: GPIO Pin

**Purpose**: Represents a microcontroller GPIO pin

**Parameters**:
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `label` | string | "GPIO" | Pin name/number |

**Visual**: Rectangle with pin label

**Anchors**:
| Anchor | Position | Direction | Purpose |
|--------|----------|-----------|---------|
| `output` | Right edge | right | Signal output |

**Sketch**:
```
┌────────┐
│ GPIO_5 ├──
└────────┘
        output
```

---

### Template: Power Symbol

**Purpose**: Represents a power rail connection (VCC)

**Parameters**:
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `voltage` | string | "VCC" | Voltage label (e.g., "+5V", "+3.3V") |

**Visual**: Horizontal line with voltage label above

**Anchors**:
| Anchor | Position | Direction | Purpose |
|--------|----------|-----------|---------|
| `rail` | Center bottom | down | Connection point |

**Sketch**:
```
  +5V
───────
   │
  rail
```

---

### Template: Ground Symbol

**Purpose**: Represents ground connection

**Parameters**: None

**Visual**: Standard ground symbol (horizontal lines decreasing in width)

**Anchors**:
| Anchor | Position | Direction | Purpose |
|--------|----------|-----------|---------|
| `gnd` | Top | up | Connection point |

**Sketch**:
```
   gnd
    │
  ─────
   ───
    ─
```

---

## Circuit Topology

### Voltage Domains

| Domain | Color Encoding | Components |
|--------|----------------|------------|
| 3.3V (GPIO compatible) | `accent-light` | GPIO pin, Gate resistor |
| 5V (Load) | `secondary-light` | LED, Current limiting resistor, Power rail |
| Ground | `foreground-3` | Ground symbol, Ground connections |

### Signal Flow

```
[GPIO] ──R1── [MOSFET Gate]
                   │
              [MOSFET]
                   │
              [Drain] ── [LED] ── R2 ── [+5V]
                   │
              [Source] ── R3 ── [GND]
```

### Layout Structure

```
col circuit {
    // Power rail (top)
    power_5v [voltage: "+5V"]

    // Load section
    row load {
        resistor r_limit [value: "220Ω"]
        led status_led [color: green]
    }

    // Driver section
    row driver {
        col gpio_section {
            gpio_pin gpio [label: "GPIO_5"]
            resistor r_gate [value: "10kΩ"]
        }
        mosfet q1
        resistor r_pulldown [value: "10kΩ"]
    }

    // Ground (bottom)
    ground gnd
}

// Connections (see spec for details)
```

---

## Connection Map

| From | To | Style | Label |
|------|----|-------|-------|
| gpio.output | r_gate.left | orthogonal | - |
| r_gate.right | q1.gate | orthogonal | - |
| q1.drain | status_led.cathode | orthogonal | - |
| status_led.anode | r_limit.right | orthogonal | - |
| r_limit.left | power_5v.rail | orthogonal | - |
| q1.source | r_pulldown.left | orthogonal | - |
| r_pulldown.right | gnd.gnd | orthogonal | - |

---

## Visual Encoding

| Concept | Encoding |
|---------|----------|
| 3.3V domain | Light accent color, dashed border |
| 5V domain | Light secondary color, solid border |
| Ground | Gray/foreground-3 |
| Signal path | Solid lines |
| Power path | Thicker lines |

---

*Created: 2026-01-28*
*Feature: 009-mosfet-driver-example*
