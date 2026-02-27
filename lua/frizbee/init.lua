--- frizbee/init.lua
--- Public Lua API for frizbee.nvim.
---
--- All functions lazily load the native backend on first call.
--- If the backend is unavailable, functions raise a clear Lua error.
---
--- API:
---   require('frizbee').match(query, texts, opts?)       -> matches
---   require('frizbee').match_indices(query, text, opts?) -> integer[]|nil
---   require('frizbee').health()                         -> table

local loader = require("frizbee.loader")

local M = {}

--- Retrieve the native backend, raising an error if unavailable.
---@return table  native frizbee_nvim module
local function backend()
  local result = loader.load()
  if not result.mod then
    local msg = table.concat({
      "frizbee.nvim: native backend 'frizbee_nvim' could not be loaded.",
      "  Error: " .. (result.err or "unknown"),
      "  Platform: " .. result.platform .. "  Extension: " .. result.ext,
      "  Searched paths:",
    }, "\n")
    for _, p in ipairs(result.searched) do
      msg = msg .. "\n    " .. p
    end
    msg = msg .. "\n  Run ':checkhealth frizbee' for details."
    msg = msg .. "\n  Build the native module: cargo build --release"
    error(msg, 2)
  end
  return result.mod
end

--- Fuzzy-match `query` against an array of strings.
---
---@param query string          The search query.
---@param texts string[]        Array of candidate strings.
---@param opts  table|nil       Optional settings:
---   - max_typos      integer|nil   Max allowed missing chars (default 0, nil = unlimited).
---   - limit          integer|nil   Max results to return (default nil = all).
---   - sort           boolean|nil   Sort by score descending (default true).
---   - with_positions boolean|nil   Include match positions in results (default false).
---   - case_sensitive boolean|nil   Case-sensitive matching (default false).
---@return table[]  matches  Array of { index, score, exact, positions? }
function M.match(query, texts, opts)
  return backend().match(query, texts, opts)
end

--- Return 1-based match positions for a single query/text pair.
---
---@param query string       The search query.
---@param text  string       A single candidate string.
---@param opts  table|nil    Same options as match() (only max_typos, case_sensitive are used).
---@return integer[]|nil     1-based positions, or nil if no match or empty query.
function M.match_indices(query, text, opts)
  return backend().match_indices(query, text, opts)
end

--- Return native module metadata. Safe to call before the backend is loaded,
--- but will return loader diagnostics instead of native info on failure.
---
---@return table  info  { backend, module, version, features } on success,
---                     or { error, platform, ext, searched } on failure.
function M.health()
  local result = loader.load()
  if result.mod then
    return result.mod.health()
  end
  -- Backend unavailable: return loader diagnostics so callers can report them.
  return {
    error = result.err,
    platform = result.platform,
    ext = result.ext,
    searched = result.searched,
  }
end

return M
