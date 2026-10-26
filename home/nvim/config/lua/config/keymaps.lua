vim.keymap.set("n", ";", ":", { noremap = true })

vim.keymap.set("n", "<C-w><C-w>", ":w!<CR>", { desc = "Save buffer" })
vim.keymap.set("n", "<C-q><C-q>", ":q!<CR>", { desc = "Quit buffer" })

vim.keymap.set("n", "<c-k>", ":wincmd k<CR>")
vim.keymap.set("n", "<c-j>", ":wincmd j<CR>")
vim.keymap.set("n", "<c-h>", ":wincmd h<CR>")
vim.keymap.set("n", "<c-l>", ":wincmd l<CR>")

vim.keymap.set("n", "<leader>sv", ":vsplit<CR>", { desc = "Vertical split" })
vim.keymap.set("n", "<leader>sh", ":split<CR>", { desc = "Horizontal split" })

vim.keymap.set("n", "<leader>hh", ":nohlsearch<CR>")

vim.keymap.set("n", "<leader>db", ":bdelete<CR>", { desc = "Delete buffer" })

vim.keymap.set("n", "<C-t>n", ":tabnew<CR>", { desc = "New tab" })
vim.keymap.set("n", "<C-t>w", ":tabclose<CR>", { desc = "Close tab" })
vim.keymap.set("n", "<Tab>", ":tabnext<CR>", { desc = "Next tab" })
vim.keymap.set("n", "<S-Tab>", ":tabprevious<CR>", { desc = "Previous tab" })

vim.keymap.set("n", "<leader>rs", ":SyncColorscheme<CR>", { desc = "Re-sync colorscheme with Ghostty" })

vim.keymap.set("n", "<leader>tt", function()
  vim.fn.jobstart("uv run --directory /Users/peki/.config/ghostty term theme random", {
    on_exit = function()
      -- Trigger colorscheme sync after theme change
      vim.schedule(function()
        vim.cmd("SyncColorscheme")
      end)
    end
  })
end, { desc = "Random terminal theme" })

-- File explorer (Neo-tree)
-- Using 'action=show' keeps cursor in current window, 'source=last' remembers which tab was active
vim.keymap.set("n", "<leader>e", ":Neotree source=last action=show toggle=true<CR>", { desc = "Toggle file explorer" })

-- Code outline (Aerial)
vim.keymap.set("n", "<leader>o", ":AerialToggle!<CR>", { desc = "Toggle code outline" })
