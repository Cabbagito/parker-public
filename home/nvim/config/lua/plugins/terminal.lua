-- Terminal plugin
-- Custom floating terminal implementation

return {
  {
    "custom-terminal",
    name = "custom-terminal",
    dir = vim.fn.stdpath("config"),
    config = function()
      local state = {
        floating = {
          buf = -1,
          win = -1,
        },
      }

      local function create_terminal(opts)
        opts = opts or {}
        -- Get the editor's dimensions
        local width = vim.o.columns
        local height = vim.o.lines

        -- Calculate window size (80% of screen)
        local win_height = math.floor(height * 0.8)
        local win_width = math.floor(width * 0.8)

        -- Calculate starting position to center the window
        local row = math.floor((height - win_height) / 2)
        local col = math.floor((width - win_width) / 2)

        -- Create a new buffer for the floating window
        local buf = nil
        if vim.api.nvim_buf_is_valid(opts.buf) then
          buf = opts.buf
        else
          buf = vim.api.nvim_create_buf(false, true)
        end

        -- Configure the window options
        local win_config = {
          relative = "editor",
          row = row,
          col = col,
          width = win_width,
          height = win_height,
          style = "minimal",
          border = "rounded",
        }

        -- Open the floating window
        local win = vim.api.nvim_open_win(buf, true, win_config)

        -- Optional: Set window-local options
        vim.wo[win].winblend = 0
        vim.wo[win].winhighlight = "Normal:Normal"
        return { win = win, buf = buf }
      end

      local toggle_terminal = function()
        if not vim.api.nvim_win_is_valid(state.floating.win) then
          state.floating = create_terminal(state.floating)
          if vim.bo[state.floating.buf].buftype ~= "terminal" then
            vim.cmd.terminal()
          end
          -- Enter insert mode when opening terminal
          vim.cmd("startinsert")
        else
          vim.api.nvim_win_hide(state.floating.win)
        end
      end

      -- Create command
      vim.api.nvim_create_user_command("Terminal", toggle_terminal, {})

      -- Keymaps
      vim.keymap.set("t", "<esc>", function()
        if vim.api.nvim_win_is_valid(state.floating.win) then
          vim.api.nvim_win_hide(state.floating.win)
        end
      end, { noremap = true, silent = true })
      vim.keymap.set("n", "<leader><leader>", ":Terminal<CR>", { noremap = true, silent = true })
    end,
  },
}