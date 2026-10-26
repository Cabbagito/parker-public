vim.g.mapleader = " "

vim.opt.termguicolors = true
vim.opt.showmode = false
vim.opt.cursorline = true
vim.opt.tabstop = 4
vim.opt.shiftwidth = 4
vim.opt.scrolloff = 8

vim.o.breakindent = true
vim.o.undofile = true
vim.o.ignorecase = true
vim.o.smartcase = true

vim.wo.relativenumber = true
vim.wo.number = true

vim.opt.clipboard = "unnamedplus"

vim.cmd("autocmd BufEnter * set formatoptions-=cro")

-- Suppress annoying notifications
vim.notify = function(msg, log_level, opts)
  -- Only show errors, suppress info/warn notifications
  if log_level == vim.log.levels.ERROR then
    vim.api.nvim_echo({{msg, "ErrorMsg"}}, true, {})
  end
end
