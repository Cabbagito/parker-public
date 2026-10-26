-- Bootstrap configuration modules
-- Load in specific order for proper initialization

-- Set leader key early
vim.g.mapleader = " "
vim.g.maplocalleader = " "

-- Core settings
require("config.options")
require("config.lazy")
require("config.keymaps")
require("config.autocmds")