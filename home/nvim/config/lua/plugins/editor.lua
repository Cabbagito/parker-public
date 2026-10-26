-- Editor enhancement plugins
-- Plugins that improve the editing experience

return {
  -- Yank/paste enhancements
  {
    "gbprod/yanky.nvim",
    event = { "BufReadPost", "BufNewFile" },
    config = function()
      require("yanky").setup({})
      
      -- Keymaps
      vim.keymap.set({ "n", "x" }, "p", "<Plug>(YankyPutAfter)")
      vim.keymap.set({ "n", "x" }, "P", "<Plug>(YankyPutBefore)")
      vim.keymap.set({ "n", "x" }, "gp", "<Plug>(YankyGPutAfter)")
      vim.keymap.set({ "n", "x" }, "gP", "<Plug>(YankyGPutBefore)")
      vim.keymap.set("n", "<C-p>", "<Plug>(YankyPreviousEntry)")
      vim.keymap.set("n", "<C-n>", "<Plug>(YankyNextEntry)")
    end,
  },

  -- Better diagnostics list
  {
    "folke/trouble.nvim",
    cmd = "Trouble",
    opts = {},
    keys = {
      { "<leader>fd", "<cmd>Trouble<CR>", desc = "Toggle diagnostics" },
    },
  },

  -- TODO comments highlight
  {
    "folke/todo-comments.nvim",
    event = { "BufReadPost", "BufNewFile" },
    opts = {},
  },

  -- Dim inactive code (focus mode)
  {
    "folke/twilight.nvim",
    cmd = "Twilight",
    keys = {
      { "<leader>tw", "<cmd>Twilight<CR>", desc = "Toggle twilight" },
    },
  },

  -- Markdown preview (was in original)
  -- Disabled in favor of render-markdown.nvim
  -- {
  --   "OXY2DEV/markview.nvim",
  --   ft = "markdown",
  --   dependencies = {
  --     "nvim-treesitter/nvim-treesitter",
  --     "nvim-tree/nvim-web-devicons",
  --   },
  -- },
}
