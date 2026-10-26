-- Navigation plugins
-- File navigation, search, and quick switching

return {
	-- Fuzzy finder
	{
		"nvim-telescope/telescope.nvim",
		dependencies = {
			"nvim-lua/plenary.nvim",
			{
				"nvim-telescope/telescope-fzf-native.nvim",
				build = "make",
			},
			"nvim-telescope/telescope-ui-select.nvim",
		},
		lazy = false, -- Load immediately for ui-select
		keys = {
			{ "<leader>ff", "<cmd>Telescope find_files<cr>",  desc = "Find File" },
			{ "<leader>fw", "<cmd>Telescope live_grep<cr>",   desc = "Find Word" },
			{ "<leader>fb", "<cmd>Telescope buffers<cr>",     desc = "Find Buffer" },
			{ "<leader>fh", "<cmd>Telescope help_tags<cr>",   desc = "Find Help" },
			{ "<leader>fr", "<cmd>Telescope oldfiles<cr>",    desc = "Find Recent File" },
			{ "<leader>fc", "<cmd>Telescope commands<cr>",    desc = "Find Command" },
			{ "<leader>ft", "<cmd>TodoTelescope<cr>",         desc = "Find Todo" },
			{ "<leader>fs", "<cmd>Telescope aerial<cr>",      desc = "Find Symbol" },
			{ "<leader>cs", "<cmd>Telescope colorscheme<cr>", desc = "Find Colorscheme" },
			{
				"<leader>nv",
				function()
					require("telescope.builtin").find_files({
						prompt_title = "Find Neovim Files",
						cwd = vim.fn.stdpath("config"),
						hidden = true,
					})
				end,
				desc = "Neovim files",
			},
			{
				"<leader>df",
				function()
					require("telescope.builtin").find_files({
						prompt_title = "Find Dotfiles",
						cwd = "~/.config/",
						hidden = true,
					})
				end,
				desc = "Dotfiles",
			},
		},
		config = function()
			require("telescope").setup({
				extensions = {
					fzf = {},
					["ui-select"] = {
						require("telescope.themes").get_dropdown({}),
					},
				},
			})
			require("telescope").load_extension("fzf")
			require("telescope").load_extension("ui-select")
			require("telescope").load_extension("aerial")
		end,
	},

	-- Quick file switching
	{
		"ThePrimeagen/harpoon",
		branch = "harpoon2",
		dependencies = { "nvim-lua/plenary.nvim" },
		keys = {
			{
				"<leader>a",
				function()
					require("harpoon"):list():add()
				end,
				desc = "Add to Harpoon",
			},
			{
				"<C-e>",
				function()
					local harpoon = require("harpoon")
					harpoon.ui:toggle_quick_menu(harpoon:list())
				end,
				desc = "Harpoon menu",
			},
			{
				"<C-1>",
				function()
					require("harpoon"):list():select(1)
				end,
				desc = "Harpoon 1",
			},
			{
				"<C-2>",
				function()
					require("harpoon"):list():select(2)
				end,
				desc = "Harpoon 2",
			},
			{
				"<C-3>",
				function()
					require("harpoon"):list():select(3)
				end,
				desc = "Harpoon 3",
			},
			{
				"<C-4>",
				function()
					require("harpoon"):list():select(4)
				end,
				desc = "Harpoon 4",
			},
			{
				"<C-5>",
				function()
					require("harpoon"):list():select(5)
				end,
				desc = "Harpoon 5",
			},
			{
				"<C-S-P>",
				function()
					require("harpoon"):list():prev()
				end,
				desc = "Harpoon prev",
			},
			{
				"<C-S-N>",
				function()
					require("harpoon"):list():next()
				end,
				desc = "Harpoon next",
			},
		},
		config = function()
			require("harpoon"):setup()
		end,
	},

	-- File manager
	{
		"stevearc/oil.nvim",
		dependencies = { "nvim-tree/nvim-web-devicons" },
		cmd = "Oil",
		keys = {
			{ "-", "<CMD>Oil<CR>", desc = "Open parent directory" },
		},
		config = function()
			require("oil").setup({
				default_file_explorer = true,
				columns = {
					"icon",
				},
				buf_options = {
					buflisted = false,
					bufhidden = "hide",
				},
				win_options = {
					wrap = false,
					signcolumn = "no",
					cursorcolumn = false,
					foldcolumn = "0",
					spell = false,
					list = false,
					conceallevel = 3,
					concealcursor = "nvic",
				},
				delete_to_trash = false,
				skip_confirm_for_simple_edits = false,
				prompt_save_on_select_new_entry = true,
				cleanup_delay_ms = 2000,
				lsp_file_methods = {
					enabled = true,
					timeout_ms = 1000,
					autosave_changes = false,
				},
				constrain_cursor = "editable",
				watch_for_changes = false,
				keymaps = {
					["g?"] = { "actions.show_help", mode = "n" },
					["<CR>"] = "actions.select",
					["<C-s>"] = { "actions.select", opts = { vertical = true } },
					["<C-h>"] = { "actions.select", opts = { horizontal = true } },
					["<C-t>"] = { "actions.select", opts = { tab = true } },
					["<C-p>"] = "actions.preview",
					["<C-c>"] = { "actions.close", mode = "n" },
					["<C-l>"] = "actions.refresh",
					["-"] = { "actions.parent", mode = "n" },
					["_"] = { "actions.open_cwd", mode = "n" },
					["`"] = { "actions.cd", mode = "n" },
					["~"] = { "actions.cd", opts = { scope = "tab" }, mode = "n" },
					["gs"] = { "actions.change_sort", mode = "n" },
					["gx"] = "actions.open_external",
					["g."] = { "actions.toggle_hidden", mode = "n" },
					["g\\"] = { "actions.toggle_trash", mode = "n" },
				},
				use_default_keymaps = true,
				view_options = {
					show_hidden = true,
					is_hidden_file = function(name, bufnr)
						local m = name:match("^%.")
						return m ~= nil
					end,
					is_always_hidden = function(name, bufnr)
						return false
					end,
					natural_order = "fast",
					case_insensitive = false,
					sort = {
						{ "type", "asc" },
						{ "name", "asc" },
					},
					highlight_filename = function(entry, is_hidden, is_link_target, is_link_orphan)
						return nil
					end,
				},
				extra_scp_args = {},
				git = {
					add = function(path)
						return false
					end,
					mv = function(src_path, dest_path)
						return false
					end,
					rm = function(path)
						return false
					end,
				},
				float = {
					padding = 2,
					max_width = 0,
					max_height = 0,
					border = "rounded",
					win_options = {
						winblend = 0,
					},
					get_win_title = nil,
					preview_split = "auto",
					override = function(conf)
						return conf
					end,
				},
				preview_win = {
					update_on_cursor_moved = true,
					preview_method = "fast_scratch",
					disable_preview = function(filename)
						return false
					end,
					win_options = {},
				},
				confirmation = {
					max_width = 0.9,
					min_width = { 40, 0.4 },
					width = nil,
					max_height = 0.9,
					min_height = { 5, 0.1 },
					height = nil,
					border = "rounded",
					win_options = {
						winblend = 0,
					},
				},
				progress = {
					max_width = 0.9,
					min_width = { 40, 0.4 },
					width = nil,
					max_height = { 10, 0.9 },
					min_height = { 5, 0.1 },
					height = nil,
					border = "rounded",
					minimized_border = "none",
					win_options = {
						winblend = 0,
					},
				},
				ssh = {
					border = "rounded",
				},
				keymaps_help = {
					border = "rounded",
				},
			})
		end,
	},
}
