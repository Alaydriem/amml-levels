namespace    = "alaydriem"
name         = "levels"
entry        = "alaydriem_levels.dll"
api_version  = 3

uses {
  feature "overlay" {
    event "client.tick" {
      version  = 1
      required = true
    }
    event "render.world_rendered" {
      version  = 1
      required = true
    }
    event "registry.provider_ready" {
      version  = 1
      required = true
    }
    event "registry.provider_gone" {
      version  = 1
      required = true
    }
    event "mods.loaded" {
      version  = 1
      required = true
    }
    event "input.lease_suspended" {
      version  = 1
      required = true
    }
    event "input.lease_resumed" {
      version  = 1
      required = true
    }
    event "input.lease_lost" {
      version  = 1
      required = true
    }
    event "input.injected_released" {
      version  = 2
      required = true
    }
    event "overlay.screenshot_taken" {
      version  = 1
      required = true
    }
    event "overlay.screenshot_written" {
      version  = 1
      required = true
    }
    area "input.lease" {
      version  = 1
      required = true
    }
    area "client.read" {
      version  = 1
      required = true
    }
    area "client.hud" {
      version  = 1
      required = true
    }
    area "actor.read" {
      version  = 3
      required = true
    }
    area "world.sky_darken" {
      version  = 1
      required = true
    }
    area "world.scan_surfaces" {
      version  = 1
      required = true
    }
    area "render.world_canvas" {
      version  = 2
      required = true
    }
  }
  feature "info" {
    event "ui.screen_render" {
      version  = 1
      required = false
    }
    area "ui.screen" {
      version  = 1
      required = false
    }
    area "ui.draw" {
      version  = 1
      required = false
    }
    area "ui.text" {
      version  = 1
      required = false
    }
    area "client.top_screen" {
      version  = 2
      required = false
    }
    area "world.chunk_read" {
      version  = 1
      required = false
    }
    area "world.biomes" {
      version  = 2
      required = false
    }
    area "world.blocks" {
      version  = 1
      required = false
    }
    area "player.hit_result" {
      version  = 1
      required = false
    }
    area "client.world_info" {
      version  = 1
      required = false
    }
    area "world.actors_in" {
      version  = 2
      required = false
    }
  }
}

listing {
  title       = "Levels"
  summary     = "Light levels, chunk borders, and spawn sphere markers."
  description = <<-EOT
    View levels and place markers in your world to see light levels, chunk borders, spawn spheres, and more.
  EOT
  authors     = ["Charles R. Portwood II"]
  license     = "BSD-3-Clause"
  source      = "https://github.com/alaydriem/amml-levels"
  homepage    = "https://github.com/alaydriem/amml-levels#readme"
  tags        = ["light-level", "spawning", "chunk-borders", "overlay", "client"]
  icon        = "icon.png"
}
