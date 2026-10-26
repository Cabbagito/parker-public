-- Miscellaneous plugins
-- Plugins that don't fit into other categories

return {
  -- Plenary (Lua functions library, dependency for many plugins)
  {
    "nvim-lua/plenary.nvim",
    lazy = true,
  },

  -- Vim practice game
  {
    "ThePrimeagen/vim-be-good",
    cmd = "VimBeGood",
  },

  -- Floating help
  {
    "Tyler-Barham/floating-help.nvim",
    cmd = { "FloatingHelp", "FloatingHelpClose", "FloatingHelpToggle" },
    config = function()
      require("floating-help").setup({
        width = 0.9,
        height = 0.9,
        position = "C",
      })
    end,
  },

  -- Autoclose brackets (disabled as in original)
  {
    "m4xshen/autoclose.nvim",
    enabled = false,
    event = "InsertEnter",
    config = function()
      require("autoclose").setup({})
    end,
  },
}