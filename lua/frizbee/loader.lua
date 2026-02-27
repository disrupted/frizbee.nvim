--- frizbee/loader.lua
--- Locates and loads the native frizbee_nvim shared library.
---
--- Strategy:
---   1. Try require('frizbee_nvim') directly (covers already-on-cpath cases).
---   2. Compute candidate paths relative to this file's plugin root.
---   3. Append each candidate pattern to package.cpath and retry.
---
--- Returns a result table:
---   {
---     mod       = <module handle | nil>,
---     err       = <string | nil>,          -- last load error
---     searched  = string[],               -- cpath patterns attempted
---     loaded_from = <string | nil>,        -- pattern that succeeded
---     platform  = "macos" | "linux" | "windows",
---     ext       = ".dylib" | ".so" | ".dll",
---   }

local M = {}

--- Detect current platform and return the expected library extension.
---@return string platform  "macos" | "linux" | "windows"
---@return string ext       ".dylib" | ".so" | ".dll"
local function detect_platform()
  local jit_os = (jit and jit.os) or ""
  if jit_os == "OSX" or jit_os == "macOS" then
    return "macos", ".dylib"
  elseif jit_os == "Windows" then
    return "windows", ".dll"
  else
    -- Linux and other Unixes
    return "linux", ".so"
  end
end

--- Resolve the plugin root from this file's location.
--- loader.lua lives at <root>/lua/frizbee/loader.lua, so root = ../../../
---@return string absolute path to the plugin root (no trailing slash)
local function plugin_root()
  -- debug.getinfo(1, "S").source is "@/abs/path/to/loader.lua"
  local src = debug.getinfo(1, "S").source
  local path = src:match("^@(.+)$") or src
  -- Strip lua/frizbee/loader.lua (3 levels)
  return path:gsub("/lua/frizbee/loader%.lua$", "")
end

--- Attempt to require the native module, returning (mod, err).
---@return table|nil mod
---@return string|nil err
local function try_require()
  local ok, result = pcall(require, "frizbee_nvim")
  if ok then
    return result, nil
  end
  return nil, tostring(result)
end

--- Build candidate cpath patterns for the given root and extension.
---@param root string  absolute plugin root path
---@param ext  string  library extension including leading dot
---@return string[]
local function candidate_patterns(root, ext)
  -- Lua's cpath uses '?' as placeholder; the loader strips the prefix up to
  -- the last path separator and maps ? -> module name.
  -- For "frizbee_nvim", the loader looks for:
  --   <dir>/frizbee_nvim<ext>   from pattern <dir>/?.so
  --   <dir>/libfrizbee_nvim<ext> is NOT directly addressed by '?' patterns;
  --   we add an explicit symlink-style pattern using "lib?" for the lib prefix.
  return {
    root .. "/target/release/?" .. ext,
    root .. "/target/release/lib?" .. ext,
    root .. "/?" .. ext,
    root .. "/lib?" .. ext,
  }
end

--- Append a pattern to package.cpath if not already present.
---@param pattern string
local function cpath_append(pattern)
  if not package.cpath:find(pattern, 1, true) then
    package.cpath = package.cpath .. ";" .. pattern
  end
end

--- Load the native frizbee_nvim module.
--- This function is idempotent: subsequent calls return the cached result.
---@return table  result  see module docstring for shape
function M.load()
  -- Fast path: already loaded by a previous call.
  if M._cache then
    return M._cache
  end

  local platform, ext = detect_platform()
  local root = plugin_root()
  local searched = {}
  local loaded_from = nil
  local last_err = nil

  -- 1. Direct require (native module already on cpath or in package.loaded)
  local mod, err = try_require()
  if mod then
    M._cache = {
      mod = mod,
      err = nil,
      searched = searched,
      loaded_from = "(default cpath)",
      platform = platform,
      ext = ext,
    }
    return M._cache
  end
  last_err = err

  -- 2. Candidate paths relative to the plugin root.
  local candidates = candidate_patterns(root, ext)
  for _, pattern in ipairs(candidates) do
    table.insert(searched, pattern)
    cpath_append(pattern)
    mod, err = try_require()
    if mod then
      loaded_from = pattern
      break
    end
    last_err = err
  end

  M._cache = {
    mod = mod,
    err = mod == nil and last_err or nil,
    searched = searched,
    loaded_from = loaded_from,
    platform = platform,
    ext = ext,
  }
  return M._cache
end

return M
