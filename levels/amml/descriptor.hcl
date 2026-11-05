namespace    = "alaydriem"
name         = "levels"
display_name = "Levels"
version      = "26.60.6"
author       = "Alaydriem"
config_path  = "config.hcl"

claim "WorldDraw" {
  mode   = "Shared"
  reason = "Light numbers, chunk borders and spawn spheres"
}

keybind "toggle" {
  label       = "Toggle light levels"
  default     = "F3+L"
  capability  = "WorldDraw"
  scope       = "Global"
  config_path = "keys.toggle"
}

keybind "borders" {
  label       = "Cycle chunk borders"
  default     = "F3+G"
  capability  = "WorldDraw"
  scope       = "Global"
  config_path = "keys.borders"
}

keybind "sphere_drop" {
  label       = "Drop spawn sphere"
  default     = "F3+J"
  capability  = "WorldDraw"
  scope       = "Global"
  config_path = "keys.sphere_drop"
}

keybind "sphere_remove" {
  label       = "Remove nearest spawn sphere"
  default     = "F3+Shift+J"
  capability  = "WorldDraw"
  scope       = "Global"
  config_path = "keys.sphere_remove"
}

keybind "sphere_clear" {
  label       = "Remove all spawn spheres"
  default     = "F3+Alt+J"
  capability  = "WorldDraw"
  scope       = "Global"
  config_path = "keys.sphere_clear"
}

section "keys" {
  title = "Keys"

  field "keys.toggle" {
    label   = "Toggle light levels"
    kind    = "chord"
    default = "F3+L"
    apply   = "Live"
  }

  field "keys.borders" {
    label   = "Cycle chunk borders"
    kind    = "chord"
    default = "F3+G"
    apply   = "Live"
  }

  field "keys.sphere_drop" {
    label   = "Drop spawn sphere"
    kind    = "chord"
    default = "F3+J"
    apply   = "Live"
  }

  field "keys.sphere_remove" {
    label   = "Remove nearest spawn sphere"
    kind    = "chord"
    default = "F3+Shift+J"
    apply   = "Live"
  }

  field "keys.sphere_clear" {
    label   = "Remove all spawn spheres"
    kind    = "chord"
    default = "F3+Alt+J"
    apply   = "Live"
  }
}

section "light" {
  title = "Light"

  field "light.red_max" {
    label   = "Red up to"
    help    = "Mobs spawn at or below this"
    kind    = { int = { min = 0, max = 15, step = 1 } }
    default = 0
    apply   = "Live"
  }

  field "light.yellow_max" {
    label   = "Yellow up to"
    kind    = { int = { min = 0, max = 15, step = 1 } }
    default = 7
    apply   = "Live"
  }
}

section "label" {
  title = "Labels"

  field "label.red" {
    label   = "Red colour"
    kind    = { color = { alpha = true, encoding = "FloatArray4" } }
    default = [1.0, 0.25, 0.25, 1.0]
    apply   = "Live"
  }

  field "label.yellow" {
    label   = "Yellow colour"
    kind    = { color = { alpha = true, encoding = "FloatArray4" } }
    default = [1.0, 0.85, 0.2, 1.0]
    apply   = "Live"
  }

  field "label.green" {
    label   = "Green colour"
    kind    = { color = { alpha = true, encoding = "FloatArray4" } }
    default = [0.35, 1.0, 0.35, 1.0]
    apply   = "Live"
  }

  field "label.lift" {
    label   = "Lift"
    help    = "Height above the block"
    kind    = { float = { min = 0.0, max = 0.05, step = 0.005 } }
    default = 0.02
    apply   = "Live"
  }

  field "label.scale" {
    label   = "Scale"
    help    = "Pixel size, 0.04 block at 1"
    kind    = { float = { min = 0.25, max = 4.0, step = 0.05 } }
    default = 1.0
    apply   = "Live"
  }
}

section "borders" {
  title = "Chunk borders"

  field "borders.line_color" {
    label   = "Line colour"
    kind    = { color = { alpha = true, encoding = "FloatArray4" } }
    default = [1.0, 1.0, 1.0, 0.6]
    apply   = "Live"
  }

  field "borders.edge_alpha" {
    label   = "Chunk edge opacity"
    kind    = { float = { min = 0.0, max = 1.0, step = 0.01 } }
    default = 0.69
    apply   = "Live"
  }

  field "borders.center_color" {
    label   = "Center line colour"
    kind    = { color = { alpha = true, encoding = "FloatArray4" } }
    default = [1.0, 1.0, 1.0, 0.6]
    apply   = "Live"
  }

  field "borders.line_width" {
    label   = "Line width"
    kind    = { float = { min = 0.01, max = 0.5, step = 0.01 } }
    default = 0.04
    apply   = "Live"
  }

  field "borders.edge_width" {
    label   = "Chunk edge width"
    kind    = { float = { min = 0.01, max = 0.5, step = 0.01 } }
    default = 0.08
    apply   = "Live"
  }

  field "borders.slime_highlight" {
    label   = "Highlight slime chunks"
    help    = "Draws the borders of slime chunks in green"
    kind    = "bool"
    default = false
    apply   = "Live"
  }
}

section "spheres" {
  title = "Spawn spheres"

  field "spheres.max_markers" {
    label   = "Maximum markers"
    kind    = { int = { min = 1, max = 64, step = 1 } }
    default = 16
    apply   = "Restart"
  }

  field "spheres.show_distance" {
    label   = "Show distance"
    kind    = { float = { min = 8.0, max = 256.0, step = 1.0 } }
    default = 96.0
    apply   = "Live"
  }

  field "spheres.inner_radius" {
    label   = "Inner radius"
    kind    = { float = { min = 1.0, max = 128.0, step = 0.5 } }
    default = 24.0
    apply   = "Live"
  }

  field "spheres.outer_radius" {
    label   = "Outer radius"
    kind    = { float = { min = 1.0, max = 128.0, step = 0.5 } }
    default = 44.5
    apply   = "Live"
  }

  field "spheres.inner_color" {
    label   = "Inner colour"
    kind    = { color = { alpha = true, encoding = "FloatArray4" } }
    default = [0.0, 1.0, 0.008, 0.08]
    apply   = "Live"
  }

  field "spheres.outer_color" {
    label   = "Outer colour"
    kind    = { color = { alpha = true, encoding = "FloatArray4" } }
    default = [1.0, 0.02, 0.008, 0.08]
    apply   = "Live"
  }

  field "spheres.spin" {
    label   = "Spin speed"
    kind    = { float = { min = -90.0, max = 90.0, step = 0.5 } }
    default = 2.5
    apply   = "Live"
  }
}

section "scan" {
  title = "Scan"

  field "scan.radius" {
    label   = "Radius (blocks)"
    kind    = { int = { min = 1, max = 32, step = 1 } }
    default = 24
    apply   = "Restart"
  }

  field "scan.pass_seconds" {
    label   = "Pass time"
    help    = "Time to rescan the whole radius"
    kind    = { float = { min = 0.05, max = 5.0, step = 0.05, unit = "s" } }
    default = 0.25
    apply   = "Restart"
  }

  field "scan.max_labels" {
    label   = "Most labels"
    help    = "Nearest are kept"
    kind    = { int = { min = 1, max = 139425, step = 256 } }
    default = 8192
    apply   = "Restart"
  }

  field "scan.max_columns_per_frame" {
    label   = "Columns per frame"
    kind    = { int = { min = 1, max = 4225, step = 16 } }
    default = 512
    apply   = "Restart"
  }
}

section "debug" {
  title = "Debug"

  field "telemetry" {
    label   = "Telemetry"
    help    = "Logs frame cost and memory every 2 s"
    kind    = "bool"
    default = false
    apply   = "Live"
  }

  field "memory_budget_mb" {
    label   = "Memory budget (MB)"
    help    = "Telemetry warns above it"
    kind    = { int = { min = 1, max = 1024, step = 1 } }
    default = 32
    apply   = "Restart"
  }
}
