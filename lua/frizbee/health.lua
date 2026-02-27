--- frizbee/health.lua
--- :checkhealth frizbee integration.
---
--- Neovim calls M.check() when the user runs :checkhealth frizbee.

local M = {}

function M.check()
  local health = vim.health

  health.start("frizbee.nvim")

  -- Load backend via loader so we get full diagnostics.
  local ok, loader = pcall(require, "frizbee.loader")
  if not ok then
    health.error("Could not load frizbee.loader: " .. tostring(loader))
    return
  end

  local result = loader.load()

  -- Platform / extension
  health.info("Platform : " .. result.platform)
  health.info("Lib ext  : " .. result.ext)

  -- Searched paths
  if #result.searched == 0 then
    health.info("Searched : (default package.cpath — no extra paths appended)")
  else
    health.info("Searched paths:")
    for _, p in ipairs(result.searched) do
      health.info("  " .. p)
    end
  end

  if result.loaded_from then
    health.info("Loaded from: " .. result.loaded_from)
  end

  -- Native module status
  if result.mod then
    health.ok("Native module 'frizbee_nvim' loaded successfully")

    -- Delegate to native health() for version / feature list.
    local hok, info = pcall(result.mod.health)
    if hok and type(info) == "table" then
      health.info("Backend  : " .. tostring(info.backend))
      health.info("Version  : " .. tostring(info.version))
      if type(info.features) == "table" then
        health.info("Features : " .. table.concat(info.features, ", "))
      end
    else
      health.warn("Native health() call failed: " .. tostring(info))
    end
  else
    health.error(
      "Native module 'frizbee_nvim' could not be loaded.\n"
        .. "  Error: "
        .. (result.err or "unknown")
        .. "\n"
        .. "  Fix: run  cargo build --release  inside the plugin directory,\n"
        .. "  then restart Neovim."
    )
  end
end

return M
