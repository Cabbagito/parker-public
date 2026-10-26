-- Code outline and symbol navigation
-- Provides tree view of code symbols (functions, classes, methods, etc.)

return {
	{
		"stevearc/aerial.nvim",
		dependencies = {
			"nvim-treesitter/nvim-treesitter",
			"nvim-tree/nvim-web-devicons",
		},
		cmd = { "AerialToggle", "AerialOpen", "AerialClose", "AerialInfo", "AerialNavToggle" },
		keys = {
			{ "<leader>o", "<cmd>AerialToggle!<CR>", desc = "Toggle code outline" },
		},
		opts = {
			-- Priority list of preferred backends for aerial
			backends = { "treesitter", "lsp", "markdown" },

			layout = {
				-- Width configuration
				max_width = { 44, 0.22 },
				width = nil,
				min_width = 10,

				-- Window options
				win_opts = {},

				-- Default direction to open aerial window
				default_direction = "right",

				-- Where aerial window will be opened (edge or window)
				placement = "edge",

				-- Resize window to fit content
				resize_to_content = true,

				-- Preserve window size equality
				preserve_equality = false,
			},

			-- How aerial window decides which buffer to display symbols for
			attach_mode = "window",

			-- List of events that trigger automatic close
			close_automatic_events = {},

			-- Keymaps in aerial window
			keymaps = {
				["?"] = "actions.show_help",
				["g?"] = "actions.show_help",
				["<CR>"] = "actions.jump",
				["<2-LeftMouse>"] = "actions.jump",
				["<C-v>"] = "actions.jump_vsplit",
				["<C-s>"] = "actions.jump_split",
				["p"] = "actions.scroll",
				["<C-j>"] = "actions.down_and_scroll",
				["<C-k>"] = "actions.up_and_scroll",
				["{"] = "actions.prev",
				["}"] = "actions.next",
				["[["] = "actions.prev_up",
				["]]"] = "actions.next_up",
				["q"] = "actions.close",
				["o"] = "actions.tree_toggle",
				["za"] = "actions.tree_toggle",
				["O"] = "actions.tree_toggle_recursive",
				["zA"] = "actions.tree_toggle_recursive",
				["l"] = "actions.tree_open",
				["zo"] = "actions.tree_open",
				["L"] = "actions.tree_open_recursive",
				["zO"] = "actions.tree_open_recursive",
				["h"] = "actions.tree_close",
				["zc"] = "actions.tree_close",
				["H"] = "actions.tree_close_recursive",
				["zC"] = "actions.tree_close_recursive",
				["zr"] = "actions.tree_increase_fold_level",
				["zR"] = "actions.tree_open_all",
				["zm"] = "actions.tree_decrease_fold_level",
				["zM"] = "actions.tree_close_all",
				["zx"] = "actions.tree_sync_folds",
				["zX"] = "actions.tree_sync_folds",
			},

			-- When true, don't load aerial until a command or function is called
			lazy_load = true,

			-- Disable aerial on files with this many lines
			disable_max_lines = 10000,

			-- Disable aerial on files this size or larger (in bytes)
			disable_max_size = 2000000, -- 2MB

			-- List of symbol kinds to display. Set to false to display all symbols.
			filter_kind = {
				"Class",
				"Constructor",
				"Enum",
				"Function",
				"Interface",
				"Module",
				"Method",
				"Struct",
				"Variable",
				"Constant",
				"Property",
				"Field",
			},

			-- Line highlighting mode when multiple splits are visible
			-- Options: split_width, full_width, last, none
			highlight_mode = "split_width",

			-- Highlight the closest symbol if cursor is not exactly on one
			highlight_closest = true,

			-- Highlight the symbol in the source buffer when cursor is in aerial window
			highlight_on_hover = false,

			-- When jumping to a symbol, highlight the line for this many ms
			-- Set to false to disable
			highlight_on_jump = 300,

			-- Jump to symbol in source window when the cursor moves
			autojump = false,

			-- Define symbol icons
			icons = {},

			-- Control which windows and buffers aerial should ignore
			ignore = {
				unlisted_buffers = false,
				diff_windows = true,
				filetypes = {},
				buftypes = "special",
				wintypes = "special",
			},

			-- Use symbol tree for folding
			manage_folds = false,

			-- When you fold code with za, zo, or zc, update the aerial tree as well
			link_folds_to_tree = false,

			-- Fold code when you open/collapse symbols in the tree
			link_tree_to_folds = true,

			-- Set default symbol icons to use patched font icons
			nerd_font = "auto",

			-- Call this function when aerial attaches to a buffer
			on_attach = function(bufnr)
				-- Jump forwards/backwards with '{' and '}'
				vim.keymap.set("n", "{", "<cmd>AerialPrev<CR>", { buffer = bufnr, desc = "Previous symbol" })
				vim.keymap.set("n", "}", "<cmd>AerialNext<CR>", { buffer = bufnr, desc = "Next symbol" })
			end,

			-- Automatically open aerial when entering supported buffers
			open_automatic = false,

			-- Run this command after jumping to a symbol
			post_jump_cmd = "normal! zz",

			-- When true, aerial will automatically close after jumping to a symbol
			close_on_select = false,

			-- The autocmds that trigger symbols update
			update_events = "TextChanged,InsertLeave",

			-- Show box drawing characters for the tree hierarchy
			show_guides = true,

			-- Customize the characters used when show_guides = true
			guides = {
				mid_item = "├─",
				last_item = "└─",
				nested_top = "│ ",
				whitespace = "  ",
			},

			-- Options for opening aerial in a floating window
			float = {
				border = "rounded",
				relative = "cursor",
				max_height = 0.9,
				height = nil,
				min_height = { 8, 0.1 },
			},

			-- Options for the floating nav windows
			nav = {
				border = "rounded",
				max_height = 0.9,
				min_height = { 10, 0.1 },
				max_width = 0.5,
				min_width = { 0.2, 20 },
				win_opts = {
					cursorline = true,
					winblend = 10,
				},
				autojump = false,
				preview = false,
				keymaps = {
					["<CR>"] = "actions.jump",
					["<2-LeftMouse>"] = "actions.jump",
					["<C-v>"] = "actions.jump_vsplit",
					["<C-s>"] = "actions.jump_split",
					["h"] = "actions.left",
					["l"] = "actions.right",
					["<C-c>"] = "actions.close",
				},
			},

			lsp = {
				-- If true, fetch document symbols when LSP diagnostics update
				diagnostics_trigger_update = false,

				-- Set to false to not update the symbols when there are LSP errors
				update_when_errors = true,

				-- How long to wait (in ms) after a buffer change before updating
				update_delay = 300,

				-- Map of LSP client name to priority. Default value is 10.
				priority = {},
			},

			treesitter = {
				-- How long to wait (in ms) after a buffer change before updating
				update_delay = 300,
			},

			markdown = {
				-- How long to wait (in ms) after a buffer change before updating
				update_delay = 300,
			},
		},
	},
}
