-- UI plugins
-- Status line, greeting screen, icons, and file explorer

return {
	-- Icons
	{
		"nvim-tree/nvim-web-devicons",
		lazy = true,
	},

	-- Status line
	{
		"nvim-lualine/lualine.nvim",
		dependencies = { "nvim-tree/nvim-web-devicons" },
		event = "VeryLazy",
		config = function()
			require("lualine").setup({
				options = {
					icons_enabled = true,
					theme = "solarized_light", -- ORIGINAL THEME: solarized_light (will be overridden by colorscheme sync)
					component_separators = { left = "", right = "" },
					section_separators = { left = "", right = "" },
					disabled_filetypes = {
						statusline = {},
						winbar = {},
					},
					ignore_focus = {},
					always_divide_middle = true,
					always_show_tabline = true,
					globalstatus = true,
					refresh = {
						statusline = 70,
						tabline = 70,
						winbar = 70,
					},
				},
				sections = {
					lualine_a = { "mode" },
					lualine_c = {
						{ "filetype" },
						{
							"aerial",
							sep = " > ", -- Breadcrumb separator
							depth = nil, -- Show all symbol levels (nil = all)
							dense = false, -- Show icons
							dense_sep = ".", -- Separator when dense mode is on
							colored = true, -- Color the symbol icons
						},
					},
					lualine_b = { { "filename", path = 4 } },
					lualine_y = { "branch" },
					lualine_x = { "diagnostics" },
					lualine_z = {
						{
							"datetime",
							style = "%H:%M",
						},
					},
				},
				tabline = {},
				winbar = {},
				inactive_winbar = {},
				extensions = {},
			})
		end,
	},

	-- Greeting screen
	{
		"goolord/alpha-nvim",
		dependencies = { "nvim-tree/nvim-web-devicons" },
		event = "VimEnter",
		config = function()
			local alpha = require("alpha")
			local dashboard = require("alpha.themes.startify")

			-- Set header position
			dashboard.section.header.opts.position = "center"

			-- Center button helper
			local function center_button(b)
				b.opts.position = "center"
				b.opts.align_shortcut = "center"
				return b
			end

			-- Configure buttons
			dashboard.section.top_buttons.val = {
				center_button(dashboard.button("e", "New file", "<cmd>ene <CR>")),
			}

			dashboard.section.bottom_buttons.val = {
				center_button(dashboard.button("q", "Quit", "<cmd>q <CR>")),
			}

			-- Center MRU sections (guarded: startify's internal mru structure
			-- drifts upstream; centering is cosmetic, never fatal)
			pcall(function()
				dashboard.section.mru.val[2].opts.position = "center"
				dashboard.section.mru_cwd.val[2].opts.position = "center"
			end)

			-- Dynamic centering based on window width
			local editor_width = vim.o.columns
			dashboard.config.opts.margin = math.floor(editor_width / 2) - 20

			vim.api.nvim_create_autocmd({ "VimResized" }, {
				callback = function()
					local editor_width = vim.o.columns
					local content_width = 20
					dashboard.config.opts.margin = math.floor(editor_width / 2) - content_width
				end,
			})

			-- Center all groups
			for _, section in ipairs(dashboard.config.layout) do
				if section.type == "group" then
					section.opts = section.opts or {}
					section.opts.position = "center"
				end
			end

			-- Custom header - DON'T PANIC (fixed)
			dashboard.section.header.val = {
				[[                                   ▄█                                                                  ]],
				[[▀███▀▀▀██▄   ▄▄█▀▀██▄ ▀███▄   ▀███▀█████▀▀██▀▀███   ▀███▀▀▀██▄      ██     ▀███▄   ▀███▀████▀ ▄▄█▀▀▀█▄█]],
				[[  ██    ▀██▄██▀    ▀██▄ ███▄    █  ▀▀█▀   ██   ▀█     ██   ▀██▄    ▄██▄      ███▄    █   ██ ▄██▀     ▀█]],
				[[  ██     ▀███▀      ▀██ █ ███   █         ██          ██   ▄██    ▄█▀██▄     █ ███   █   ██ ██▀       ▀]],
				[[  ██      ███        ██ █  ▀██▄ █         ██          ███████    ▄█  ▀██     █  ▀██▄ █   ██ ██         ]],
				[[  ██     ▄███▄      ▄██ █   ▀██▄█         ██          ██         ████████    █   ▀██▄█   ██ ██▄        ]],
				[[  ██    ▄██▀██▄    ▄██▀ █     ███         ██          ██        █▀      ██   █     ███   ██ ▀██▄     ▄▀]],
				[[▄████████▀   ▀▀████▀▀ ▄███▄    ██       ▄████▄      ▄████▄    ▄███▄   ▄████▄███▄    ██ ▄████▄ ▀▀█████▀]],
			}

			-- Adjust vertical padding
			dashboard.config.layout[1].val = 5

			-- Set highlight group (color will be set by colorscheme logic)
			dashboard.section.header.opts.hl = "DashboardLogo"

			alpha.setup(dashboard.opts)

			-- Set keymap
			vim.keymap.set("n", "<leader>hs", ":Alpha<CR>", { desc = "Homescreen" })
		end,
	},
}
