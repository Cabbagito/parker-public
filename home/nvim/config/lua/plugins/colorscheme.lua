-- Colorscheme plugins
-- All theme-related plugins and their configurations

-- MISSING THEMES FROM GHOSTTY FAVORITES (51% not covered):
-- These themes are available in Ghostty but don't have Neovim equivalents yet:
-- 
-- THEMES WITH AVAILABLE NEOVIM PLUGINS (can be added):
-- - GitHub Dark → projekt0n/github-nvim-theme (supports github_dark variant)
-- - Everforest Dark - Hard → sainnhe/everforest (already added, needs hard variant config)
-- - MaterialOcean → marko-cerovac/material.nvim (already added, needs ocean variant)
-- - Oceanic-Next → mhartington/oceanic-next
-- - nord-wave → shaunsingh/nord.nvim or arcticicestudio/nord-vim
-- - Espresso → gmoe/vim-espresso
-- - Hybrid → w0ng/vim-hybrid
-- - Snazzy → connorholyday/vim-snazzy
-- - xcodewwdc → lunacookies/vim-colors-xcode (already added, needs wwdc variant)
--
-- TERMINAL-SPECIFIC THEMES WITHOUT KNOWN NEOVIM PORTS:
-- - Andromeda
-- - Banana Blueberry
-- - Blazer
-- - Builtin Solarized Dark
-- - Chalkboard
-- - Chester
-- - CobaltNext-Dark
-- - CobaltNext-Minimal
-- - Darkside
-- - Earthsong
-- - FrontEndDelight
-- - Galaxy
-- - Guezwhoz
-- - Havn Skumring
-- - Hivacruz
-- - Hopscotch.256
-- - Japanesque
-- - Lab Fox
-- - Later This Evening
-- - LiquidCarbon
-- - Misterioso
-- - Monokai Soda
-- - Obsidian
-- - OceanicMaterial
-- - Overnight Slumber
-- - Paraiso Dark
-- - PencilDark
-- - Popping and Locking
-- - Rapture
-- - Rippedcasts
-- - SeaShells
-- - Sublette
-- - Ubuntu
-- - Vaughn
-- - Whimsy
-- - lovelace
-- - purplepeter
--
-- COVERAGE: Currently ~49% of Ghostty favorites are supported

-- Frequently used themes that should always be loaded
local always_load = {
	"catppuccin",
	"tokyonight",
	"flexoki",
	"kanagawa",
	"rose-pine",
	"nightfox",
	"dracula",
}

-- Helper function for catppuccin highlights
local function apply_catppuccin_highlights()
	vim.cmd([[
		" Set current line number to orange
		highlight CursorLineNr ctermfg=214 guifg=#ffaf00
		" Set normal line numbers to be more visible
		highlight LineNr ctermfg=250 guifg=#bcbcbc
	]])
end

-- Universal transparency enforcement function
local function enforce_transparency()
	-- Clear backgrounds for main highlight groups
	vim.cmd([[
		highlight Normal guibg=NONE ctermbg=NONE
		highlight NormalFloat guibg=NONE ctermbg=NONE
		highlight NormalNC guibg=NONE ctermbg=NONE
		highlight SignColumn guibg=NONE ctermbg=NONE
		highlight EndOfBuffer guibg=NONE ctermbg=NONE
		highlight Terminal guibg=NONE ctermbg=NONE
		highlight VertSplit guibg=NONE ctermbg=NONE
		highlight Folded guibg=NONE ctermbg=NONE
		highlight FoldColumn guibg=NONE ctermbg=NONE
		highlight ColorColumn guibg=NONE ctermbg=NONE
		highlight LineNr guibg=NONE ctermbg=NONE
		highlight CursorLineNr guibg=NONE ctermbg=NONE
		highlight NonText guibg=NONE ctermbg=NONE
		highlight SpecialKey guibg=NONE ctermbg=NONE
		highlight StatusLine guibg=NONE ctermbg=NONE
		highlight StatusLineNC guibg=NONE ctermbg=NONE
		
		" Floating windows
		highlight Pmenu guibg=NONE ctermbg=NONE
		highlight PmenuSel guibg=NONE ctermbg=NONE
		highlight PmenuSbar guibg=NONE ctermbg=NONE
		highlight PmenuThumb guibg=NONE ctermbg=NONE
		
		" Diagnostic backgrounds
		highlight DiagnosticVirtualTextError guibg=NONE ctermbg=NONE
		highlight DiagnosticVirtualTextWarn guibg=NONE ctermbg=NONE
		highlight DiagnosticVirtualTextInfo guibg=NONE ctermbg=NONE
		highlight DiagnosticVirtualTextHint guibg=NONE ctermbg=NONE
	]])
end

-- Function to get a prominent color from the current colorscheme
local function get_prominent_color()
	-- Helper to convert decimal to hex
	local function dec_to_hex(dec)
		if dec then
			return string.format("#%06x", dec)
		end
	end
	
	-- List of highlight groups to check, in order of preference
	local highlight_groups = {
		"Function",      -- Usually blue/purple/cyan
		"Keyword",       -- Often red/pink/orange
		"Type",          -- Often green/yellow
		"String",        -- Often green/yellow/orange
		"Special",       -- Usually a standout color
		"Statement",     -- Bold colors
		"Constant",      -- Distinct colors
		"Identifier",    -- Variable colors
		"PreProc",       -- Preprocessor, often bright
		"Number",        -- Number literals
		"Title",         -- Titles are often prominent
	}
	
	-- Try each highlight group until we find one with a foreground color
	for _, group in ipairs(highlight_groups) do
		local ok, hl = pcall(vim.api.nvim_get_hl_by_name, group, true)
		if ok and hl.foreground then
			return dec_to_hex(hl.foreground)
		end
	end
	
	-- Fallback to orange if nothing found
	return "#ee9550"
end

-- Manual theme mapping for lualine per colorscheme
local lualine_themes = {
	-- Catppuccin variants (has built-in lualine support)
	["catppuccin-frappe"] = "solarized_light",
	["catppuccin-latte"] = "solarized_light",
	["catppuccin-macchiato"] = "solarized_light",
	["catppuccin-mocha"] = "solarized_light",
	["catppuccin"] = "solarized_light",
	
	-- Tokyo Night variants (has built-in lualine support)
	["tokyonight"] = "solarized_light",
	["tokyonight-storm"] = "solarized_light",
	["tokyonight-moon"] = "solarized_light",
	["tokyonight-day"] = "solarized_light",
	
	-- Direct mappings
	["dracula"] = "dracula",
	["gruvbox"] = "gruvbox",
	["rose-pine"] = "solarized_light",
	["rose-pine-moon"] = "solarized_light",
	["nightfox"] = "nightfly",
	["carbonfox"] = "nightfly",
	["terafox"] = "solarized_light",
	["ayu"] = "ayu",
	["ayu-mirage"] = "ayu_mirage",
	["everblush"] = "solarized_light",
	["jellybeans-nvim"] = "jellybeans",
	["jellybeans"] = "jellybeans",
	["aura"] = "solarized_light",
	["solarized-osaka"] = "solarized_dark",
	["iceberg"] = "iceberg",
	["neobones"] = "solarized_light",
	["monokai-pro"] = "molokai",
	["material"] = "material",
	["palenight"] = "palenight",
	["flexoki"] = "solarized_light",
	["kanagawa"] = "solarized_light",
	["kanagawa-wave"] = "solarized_light",
	["everforest"] = "everforest",
	["edge"] = "solarized_light",
	["sonokai"] = "solarized_light",
	["horizon"] = "horizon",
	["synthwave84"] = "solarized_light",
	["vesper"] = "solarized_light",
	["xcode"] = "codedark",
	["challenger_deep"] = "solarized_light",
	["cyberdream"] = "solarized_light",
	["purple-rain"] = "solarized_light",
	["SpacegrayEighties"] = "solarized_light",
	["spacedust"] = "solarized_light",
	
	-- Base16 themes
	["base16-rebecca"] = "base16",
	["base16-hopscotch"] = "base16",
	["base16-spacedust"] = "base16",
}

-- Manual color mappings for dashboard logo per colorscheme
local dashboard_colors = {
	-- Catppuccin variants (using Peach/Orange)
	["catppuccin-frappe"] = "#ef9f76",     -- Peach
	["catppuccin-latte"] = "#fe640b",      -- Peach
	["catppuccin-macchiato"] = "#f5a97f",  -- Peach
	["catppuccin-mocha"] = "#fab387",      -- Peach
	["catppuccin"] = "#f5a97f",            -- Default to macchiato peach
	
	-- Ayu
	["ayu"] = "#ffcc66",                   -- Orange
	["ayu-mirage"] = "#ffcc66",            -- Orange
	
	-- Additional themes
	["everblush"] = "#e57474",             -- Red
	["jellybeans"] = "#8197bf",            -- Blue
	["aura"] = "#a277ff",                  -- Purple
	["solarized-osaka"] = "#268bd2",       -- Blue
	["iceberg"] = "#84a0c6",               -- Blue
	["neobones"] = "#bb9584",              -- Brown
	["spacegray"] = "#b04b57",             -- Red
	["spacedust"] = "#ecf0c1",             -- Light green
	["palenight"] = "#c792ea",             -- Purple
	
	-- Tokyo Night variants
	["tokyonight"] = "#7aa2f7",            -- Blue
	["tokyonight-storm"] = "#7aa2f7",      -- Blue
	["tokyonight-moon"] = "#82aaff",       -- Blue
	["tokyonight-day"] = "#0969da",        -- Blue
	
	-- Other themes
	["dracula"] = "#bd93f9",               -- Purple
	["gruvbox"] = "#fe8019",               -- Orange
	["kanagawa"] = "#957fb8",              -- Purple (oniViolet)
	["kanagawa-wave"] = "#957fb8",         -- Purple
	["rose-pine"] = "#ebbcba",             -- Rose
	["rose-pine-moon"] = "#ea9a97",        -- Rose
	["rose-pine-dawn"] = "#d7827e",        -- Rose
	["nightfox"] = "#81a1c1",              -- Blue
	["carbonfox"] = "#78a9ff",             -- Blue
	["terafox"] = "#73daca",               -- Teal
	["flexoki"] = "#e6652e",               -- Orange
	["everforest"] = "#a7c080",            -- Green
	["edge"] = "#6cb6eb",                  -- Blue
	["sonokai"] = "#fc5d7c",               -- Red
	["monokai-pro"] = "#fc9867",           -- Orange
	["material"] = "#89ddff",              -- Cyan
	["horizon"] = "#e95678",               -- Red
	["synthwave84"] = "#ff7edb",           -- Pink
	["cyberdream"] = "#ff6e5e",            -- Red
	["vesper"] = "#8be9fd",                -- Cyan
	["challenger_deep"] = "#ffb378",       -- Orange
	["xcode"] = "#fc5fa3",                 -- Pink
}

-- Function to update dashboard logo color
local function update_dashboard_logo_color()
	local current_colorscheme = vim.g.colors_name
	local color = dashboard_colors[current_colorscheme]
	
	-- If no manual mapping exists, use dynamic detection
	if not color then
		color = get_prominent_color()
	end
	
	vim.api.nvim_set_hl(0, "DashboardLogo", { fg = color })
end

-- Function to update lualine theme based on colorscheme
local function update_lualine_theme()
	local current_colorscheme = vim.g.colors_name
	if not current_colorscheme then
		return
	end
	
	-- Get the mapped lualine theme or use 'solarized_light' as fallback
	local lualine_theme = lualine_themes[current_colorscheme] or "solarized_light"
	
	-- Update lualine configuration
	local ok, lualine = pcall(require, "lualine")
	if ok then
		-- Get current config and update only the theme
		local config = require("lualine").get_config()
		config.options.theme = lualine_theme
		lualine.setup(config)
	end
end

-- Theme mapping handlers for special cases
local theme_handlers = {
	-- Tokyo Night variants
	["tokyonight-storm"] = function()
		require("tokyonight").setup({ style = "storm", transparent = true })
		return "tokyonight"
	end,
	["tokyonight-moon"] = function()
		require("tokyonight").setup({ style = "moon", transparent = true })
		return "tokyonight"
	end,
	["tokyonight_moon"] = function()
		require("tokyonight").setup({ style = "moon", transparent = true })
		return "tokyonight"
	end,
	["tokyonight-day"] = function()
		require("tokyonight").setup({ style = "day", transparent = true })
		return "tokyonight"
	end,
	["tokyonight"] = function()
		require("tokyonight").setup({ style = "night", transparent = true })
		return "tokyonight"
	end,
	["TokyoNight"] = function()
		require("tokyonight").setup({ style = "night", transparent = true })
		return "tokyonight"
	end,
	["TokyoNight Night"] = function()
		require("tokyonight").setup({ style = "night", transparent = true })
		return "tokyonight"
	end,
	["TokyoNight Storm"] = function()
		require("tokyonight").setup({ style = "storm", transparent = true })
		return "tokyonight"
	end,
	["TokyoNight Moon"] = function()
		require("tokyonight").setup({ style = "moon", transparent = true })
		return "tokyonight"
	end,
	["TokyoNight Day"] = function()
		require("tokyonight").setup({ style = "day", transparent = true })
		return "tokyonight"
	end,

	-- Catppuccin variants
	["catppuccin-frappe"] = function()
		vim.cmd("Catppuccin macchiato")  -- Use macchiato instead of frappe
		apply_catppuccin_highlights()
		enforce_transparency()
	end,
	["catppuccin-latte"] = function()
		vim.cmd("Catppuccin latte")
		apply_catppuccin_highlights()
		enforce_transparency()
	end,
	["catppuccin-macchiato"] = function()
		vim.cmd("Catppuccin macchiato")
		apply_catppuccin_highlights()
		enforce_transparency()
	end,
	["catppuccin-mocha"] = function()
		vim.cmd("Catppuccin mocha")
		apply_catppuccin_highlights()
		enforce_transparency()
	end,
	["Catppuccin Frappe"] = function()
		vim.cmd("Catppuccin macchiato")  -- Use macchiato instead of frappe
		apply_catppuccin_highlights()
		enforce_transparency()
	end,
	["Catppuccin Latte"] = function()
		vim.cmd("Catppuccin latte")
		apply_catppuccin_highlights()
		enforce_transparency()
	end,
	["Catppuccin Macchiato"] = function()
		vim.cmd("Catppuccin macchiato")
		apply_catppuccin_highlights()
		enforce_transparency()
	end,
	["Catppuccin Mocha"] = function()
		vim.cmd("Catppuccin mocha")
		apply_catppuccin_highlights()
		enforce_transparency()
	end,

	-- Rose Pine variants
	["rose-pine"] = function()
		require("rose-pine").setup({ variant = "main", disable_background = true })
		return "rose-pine"
	end,
	["rose-pine-moon"] = function()
		require("rose-pine").setup({ variant = "moon", disable_background = true })
		return "rose-pine"
	end,
	["rose-pine-dawn"] = function()
		require("rose-pine").setup({ variant = "dawn", disable_background = true })
		return "rose-pine"
	end,
	["Rose Pine"] = function()
		require("rose-pine").setup({ variant = "main", disable_background = true })
		return "rose-pine"
	end,
	["Rose Pine Moon"] = function()
		require("rose-pine").setup({ variant = "moon", disable_background = true })
		return "rose-pine"
	end,
	["Rose Pine Dawn"] = function()
		require("rose-pine").setup({ variant = "dawn", disable_background = true })
		return "rose-pine"
	end,

	-- Nightfox family
	["nightfox"] = function()
		return "nightfox"
	end,
	["Nightfox"] = function()
		return "nightfox"
	end,
	["carbonfox"] = function()
		return "carbonfox"
	end,
	["terafox"] = function()
		return "terafox"
	end,
	["Terafox"] = function()
		return "terafox"
	end,
	["Dayfox"] = function()
		return "dayfox"
	end,

	-- Simple mappings
	["flexoki-dark"] = function()
		return "flexoki"
	end,
	["Flexoki Dark"] = function()
		return "flexoki"
	end,
	["GruvboxDark"] = function()
		return "gruvbox"
	end,
	["gruvbox-dark"] = function()
		return "gruvbox"
	end,
	["Gruvbox Dark"] = function()
		return "gruvbox"
	end,
	["Gruvbox Dark Hard"] = function()
		return "gruvbox"
	end,
	["Dracula+"] = function()
		return "dracula"
	end,
	["Dracula"] = function()
		return "dracula"
	end,
	["kanagawabones"] = function()
		return "kanagawa"
	end,
	["Kanagawabones"] = function()
		return "kanagawa"
	end,
	["Kanagawa Wave"] = function()
		return "kanagawa-wave"
	end,
	
	-- Additional theme mappings
	["ChallengerDeep"] = function()
		return "challenger_deep"
	end,
	["cyberpunk"] = function()
		return "cyberdream"
	end,
	["CyberpunkScarletProtocol"] = function()
		return "cyberdream"
	end,
	["Scarlet Protocol"] = function()
		return "cyberdream"
	end,
	["Horizon"] = function()
		return "horizon"
	end,
	["synthwave-everything"] = function()
		return "synthwave84"
	end,
	["vesper"] = function()
		return "vesper"
	end,
	["xcodedark"] = function()
		vim.g.xcode_style = "dark"
		return "xcode"
	end,
	
	-- Additional mappings for remaining favorites
	["Purple Rain"] = function()
		return "purple-rain"
	end,
	["rebecca"] = function()
		return "base16-rebecca"
	end,
	-- ["SpaceGray Eighties"] = function()
	-- 	return "spacegray"
	-- end,
	["Monokai Pro Octagon"] = function()
		require("monokai-pro").setup({
			transparent_background = true,
			filter = "octagon",
		})
		return "monokai-pro"
	end,
	["Hopscotch"] = function()
		return "base16-hopscotch"
	end,
	["Spacedust"] = function()
		return "base16-spacedust"
	end,
	["Monokai Pro Spectrum"] = function()
		require("monokai-pro").setup({
			transparent_background = true,
			filter = "spectrum",
		})
		return "monokai-pro"
	end,
	
	-- Ayu variants (both old and new Ghostty naming)
	["Ayu"] = function()
		require("ayu").setup({ mirage = false })
		return "ayu"
	end,
	["Ayu Light"] = function()
		require("ayu").setup({ mirage = false })
		return "ayu"
	end,
	["Ayu Mirage"] = function()
		require("ayu").setup({ mirage = true })
		return "ayu"
	end,
	
	-- Everblush
	["Everblush"] = function()
		return "everblush"
	end,
	
	-- Jellybeans
	["Jellybeans"] = function()
		return "jellybeans-nvim"
	end,
	
	-- Aura
	["Aura"] = function()
		return "aura"
	end,
	
	-- Solarized variants (both old and new Ghostty naming)
	["solarized-osaka-night"] = function()
		return "solarized-osaka"
	end,
	["Solarized Osaka Night"] = function()
		return "solarized-osaka"
	end,
	["Solarized Dark Patched"] = function()
		return "solarized-osaka"
	end,
	["Solarized Dark - Patched"] = function()
		return "solarized-osaka"
	end,
	["Solarized Dark Higher Contrast"] = function()
		return "solarized-osaka"
	end,
	["Builtin Solarized Dark"] = function()
		return "solarized-osaka"
	end,
	["Builtin Solarized Light"] = function()
		return "solarized-osaka"
	end,
	
	-- Iceberg (both old and new Ghostty naming)
	["iceberg-dark"] = function()
		return "iceberg"
	end,
	["Iceberg Dark"] = function()
		return "iceberg"
	end,
	["Iceberg Light"] = function()
		return "iceberg"
	end,
	
	-- Neobones from Zenbones (both old and new Ghostty naming)
	["neobones_dark"] = function()
		return "neobones"
	end,
	["Neobones Dark"] = function()
		return "neobones"
	end,
	["Neobones Light"] = function()
		return "neobones"
	end,
	["Zenbones"] = function()
		return "zenbones"
	end,
	["Zenbones Dark"] = function()
		return "zenbones"
	end,
	["Zenbones Light"] = function()
		return "zenbones"
	end,
	
	-- Kanagawa Dragon
	["Kanagawa Dragon"] = function()
		require("kanagawa").setup({ theme = "dragon" })
		return "kanagawa"
	end,
	
	-- SpaceGray variants (both old and new Ghostty naming)
	["SpaceGray"] = function()
		return "SpacegrayEighties"
	end,
	["Spacegray"] = function()
		return "SpacegrayEighties"
	end,
	["Spacegray Bright"] = function()
		return "SpacegrayEighties"
	end,
	["SpaceGray Eighties"] = function()
		return "SpacegrayEighties"
	end,
	["Spacegray Eighties"] = function()
		return "SpacegrayEighties"
	end,
	["Spacegray Eighties Dull"] = function()
		return "SpacegrayEighties"
	end,
	
	-- Spacedust
	["Spacedust"] = function()
		return "spacedust"
	end,
	
	-- Palenight (both old and new Ghostty naming)
	["PaleNightHC"] = function()
		return "palenight"
	end,
	["Pale Night Hc"] = function()
		return "palenight"
	end,

	-- Additional themes from Ghostty list that may work
	["Everforest Dark   Hard"] = function()
		vim.g.everforest_background = "hard"
		return "everforest"
	end,
	["Everforest Light   Med"] = function()
		vim.g.everforest_background = "medium"
		return "everforest"
	end,
	["Material"] = function()
		return "material"
	end,
	["Material Dark"] = function()
		vim.g.material_style = "darker"
		return "material"
	end,
	["Material Darker"] = function()
		vim.g.material_style = "darker"
		return "material"
	end,
	["Material Ocean"] = function()
		vim.g.material_style = "oceanic"
		return "material"
	end,
	["Edge"] = function()
		return "edge"
	end,
	["Sonokai"] = function()
		return "sonokai"
	end,
	["Nord"] = function()
		return "nord"
	end,
	["Nord Light"] = function()
		return "nord"
	end,
	["Nord Wave"] = function()
		return "nord"
	end,
}

-- Helper function to check if a theme is frequently used
local function is_frequent_theme(name)
	for _, theme in ipairs(always_load) do
		if name:find(theme) then
			return true
		end
	end
	return false
end

return {
	-- Ghostty theme sync plugin
	{
		"landerson02/ghostty-theme-sync.nvim",
		lazy = false,
		priority = 1001,
		config = function()
			local sync = require("ghostty-theme-sync")
			sync.setup({
				ghostty_config_path = "~/.config/ghostty/config",
			})

			-- Create a custom sync function that uses our handlers
			local function custom_sync()
				-- Helper function to safely set colorscheme
				local function safe_colorscheme(name)
					if not name or name == "default" or name == "" then
						return false
					end
					local ok = pcall(vim.cmd.colorscheme, name)
					-- Check if it actually set and not "default"
					if ok and vim.g.colors_name ~= "default" then
						return true
					end
					return false
				end
				
				-- First try the plugin's sync
				local ok = pcall(sync.sync_theme)
				
				-- If the plugin handled it successfully and didn't set "default", we're done
				if ok and vim.g.colors_name and vim.g.colors_name ~= "default" then
					return
				end

				-- Otherwise, use our custom logic
				local config_path = vim.fn.expand("~/.config/ghostty/config")
				local file_exists = vim.fn.filereadable(config_path) == 1
				
				if not file_exists then
					-- No ghostty config, use fallback
					vim.cmd("Catppuccin macchiato")
					apply_catppuccin_highlights()
					enforce_transparency()
					return
				end
				
				local lines = vim.fn.readfile(config_path)
				local ghostty_theme = nil

				for _, line in ipairs(lines) do
					local theme = line:match("^theme%s*=%s*(.+)$")
					if theme then
						ghostty_theme = vim.trim(theme)
						break
					end
				end

				if ghostty_theme then
					-- Check if we have a handler for this theme
					local handler = theme_handlers[ghostty_theme]
					if handler then
						local colorscheme = handler()
						if colorscheme and safe_colorscheme(colorscheme) then
							return
						end
					end

					-- Try direct colorscheme match
					if safe_colorscheme(ghostty_theme) then
						return
					end

					-- Try lowercase version
					local lower_theme = ghostty_theme:lower()
					if safe_colorscheme(lower_theme) then
						return
					end

					-- Try removing common suffixes
					local base_theme = ghostty_theme:gsub("%-dark$", ""):gsub("%-light$", "")
					if safe_colorscheme(base_theme) then
						return
					end
					
					-- Try replacing spaces with hyphens
					local hyphen_theme = ghostty_theme:gsub("%s+", "-"):lower()
					if safe_colorscheme(hyphen_theme) then
						return
					end
				end

				-- Always fallback to catppuccin macchiato if nothing else works
				vim.cmd("Catppuccin macchiato")
				apply_catppuccin_highlights()
				enforce_transparency()
			end

			-- Use our custom sync on startup (synchronously to avoid flicker)
			custom_sync()
			
			-- Make custom_sync available globally for manual triggering
			_G.sync_colorscheme = function()
				custom_sync()
			end
			
			-- Create user command
			vim.api.nvim_create_user_command("SyncColorscheme", function()
				_G.sync_colorscheme()
			end, { desc = "Sync colorscheme with Ghostty terminal" })
			
			-- Set up autocmd to enforce transparency on any colorscheme change
			vim.api.nvim_create_autocmd("ColorScheme", {
				pattern = "*",
				callback = function()
					vim.defer_fn(function()
						enforce_transparency()
						update_dashboard_logo_color()
						update_lualine_theme()
					end, 1)
				end,
				desc = "Enforce transparency, update dashboard color, and sync lualine theme after colorscheme change",
			})
			
			-- Apply transparency, dashboard color, and lualine theme immediately
			enforce_transparency()
			update_dashboard_logo_color()
			update_lualine_theme()
		end,
		dependencies = {
			"catppuccin/nvim",
		},
	},

	-- Catppuccin (primary theme and fallback)
	{
		"catppuccin/nvim",
		name = "catppuccin",
		lazy = false,
		priority = 1000,
		config = function()
			require("catppuccin").setup({
				flavour = "macchiato",
				background = {
					light = "macchiato",
					dark = "macchiato",
				},
				transparent_background = true,
				show_end_of_buffer = true,
				term_colors = true,
				dim_inactive = {
					enabled = false,
					shade = "dark",
					percentage = 0.15,
				},
				no_italic = false,
				no_bold = false,
				no_underline = false,
				styles = {
					comments = { "italic" },
					conditionals = { "italic" },
					loops = {},
					functions = {},
					keywords = {},
					strings = {},
					variables = {},
					numbers = {},
					booleans = {},
					properties = {},
					types = {},
					operators = {},
				},
				color_overrides = {},
				custom_highlights = {},
				default_integrations = true,
				integrations = {
					cmp = true,
					gitsigns = true,
					nvimtree = true,
					treesitter = true,
					notify = false,
					mini = {
						enabled = true,
						indentscope_color = "",
					},
				},
			})

			-- Custom highlights
			vim.cmd([[
        " Set current line number to orange
        highlight CursorLineNr ctermfg=214 guifg=#ffaf00

        " Set normal line numbers to be more visible
        highlight LineNr ctermfg=250 guifg=#bcbcbc
      ]])
		end,
	},

	-- Tokyo Night
	{
		"folke/tokyonight.nvim",
		lazy = false,
		priority = 999,
		config = function()
			require("tokyonight").setup({
				style = "storm",
				light_style = "day",
				transparent = true,
				terminal_colors = true,
			})
		end,
	},

	-- Dracula
	{
		"Mofiqul/dracula.nvim",
		lazy = false,
		priority = 999,
		config = function()
			require("dracula").setup({
				transparent_bg = true,
			})
		end,
	},

	-- Flexoki
	{
		"kepano/flexoki-neovim",
		lazy = false,
		priority = 999,
		name = "flexoki",
	},

	-- Gruvbox
	{
		"ellisonleao/gruvbox.nvim",
		lazy = not is_frequent_theme("gruvbox"),
		priority = 999,
		config = function()
			require("gruvbox").setup({
				transparent_mode = true,
			})
		end,
	},

	-- Kanagawa
	{
		"rebelot/kanagawa.nvim",
		lazy = false,
		priority = 999,
		config = function()
			require("kanagawa").setup({
				transparent = true,
			})
		end,
	},

	-- Nightfox (includes nightfox, carbonfox, terafox)
	{
		"EdenEast/nightfox.nvim",
		lazy = false,
		priority = 999,
		config = function()
			require("nightfox").setup({
				options = {
					transparent = true,
				},
			})
		end,
	},

	-- Rose Pine
	{
		"rose-pine/neovim",
		name = "rose-pine",
		lazy = false,
		priority = 999,
		config = function()
			require("rose-pine").setup({
				disable_background = true,
			})
		end,
	},

	-- Monokai Pro
	{
		"loctvl842/monokai-pro.nvim",
		lazy = not is_frequent_theme("monokai"),
		priority = 999,
		config = function()
			require("monokai-pro").setup({
				transparent_background = true,
			})
		end,
	},

	-- Material (might work for some themes)
	{
		"marko-cerovac/material.nvim",
		lazy = not is_frequent_theme("material"),
		priority = 999,
		config = function()
			vim.g.material_style = "darker"
			vim.g.material_transparent = true
		end,
	},

	-- Everforest
	{
		"sainnhe/everforest",
		lazy = true,
		config = function()
			vim.g.everforest_transparent_background = 1
			vim.g.everforest_better_performance = 1
		end,
	},

	-- Edge
	{
		"sainnhe/edge",
		lazy = true,
		config = function()
			vim.g.edge_transparent_background = 1
			vim.g.edge_better_performance = 1
		end,
	},

	-- Sonokai
	{
		"sainnhe/sonokai",
		lazy = true,
		config = function()
			vim.g.sonokai_transparent_background = 1
			vim.g.sonokai_style = "andromeda"
			vim.g.sonokai_better_performance = 1
		end,
	},

	-- Additional colorschemes for missing favorites
	{
		"challenger-deep-theme/vim",
		name = "challenger-deep-theme",
		lazy = true,
		config = function()
			vim.g.challenger_deep_termcolors = 256
			vim.g.challenger_deep_transparent = true
		end,
	},

	{
		"scottmckendry/cyberdream.nvim",
		lazy = true,
		config = function()
			require("cyberdream").setup({
				transparent = true,
			})
		end,
	},

	{
		"lunarvim/horizon.nvim",
		lazy = true,
		config = function()
			vim.g.horizon_transparent = true
		end,
	},

	{
		"lunarvim/synthwave84.nvim",
		lazy = true,
		config = function()
			vim.g.synthwave_transparent = true
		end,
	},

	{
		"datsfilipe/vesper.nvim",
		lazy = true,
		config = function()
			require("vesper").setup({
				transparent = true,
			})
		end,
	},

	{
		"lunacookies/vim-colors-xcode",
		lazy = true,
	},

	-- Purple Rain theme
	{
		"yashranjan1/purple-rain.nvim",
		lazy = true,
		config = function()
			require("purple-rain").setup({
				transparent = true,
			})
		end,
	},

	-- Ayu theme
	{
		"Shatur/neovim-ayu",
		lazy = true,
		config = function()
			require("ayu").setup({
				mirage = true,
				overrides = {
					Normal = { bg = "None" },
					ColorColumn = { bg = "None" },
					SignColumn = { bg = "None" },
					Folded = { bg = "None" },
					FoldColumn = { bg = "None" },
					CursorLine = { bg = "None" },
					CursorColumn = { bg = "None" },
					WhichKeyFloat = { bg = "None" },
					VertSplit = { bg = "None" },
				},
			})
		end,
	},

	-- Everblush theme
	{
		"Everblush/nvim",
		name = "everblush",
		lazy = true,
		config = function()
			-- Everblush doesn't have a setup function, just set colorscheme
			vim.g.everblush_transparent = true
		end,
	},

	-- Jellybeans theme
	{
		"metalelf0/jellybeans-nvim",
		lazy = true,
		dependencies = { "rktjmp/lush.nvim" },
	},

	-- Aura theme: removed — techtuner/aura-neovim was deleted upstream (404,
	-- lazy.nvim clone hangs on a credential prompt). Mapping entries for
	-- "Aura" remain but the scheme is unavailable.

	-- Solarized Osaka theme
	{
		"craftzdog/solarized-osaka.nvim",
		lazy = true,
		config = function()
			require("solarized-osaka").setup({
				transparent = true,
			})
		end,
	},

	-- Iceberg theme
	{
		"cocopon/iceberg.vim",
		lazy = true,
	},

	-- Zenbones (includes neobones)
	{
		"zenbones-theme/zenbones.nvim",
		lazy = true,
		dependencies = { "rktjmp/lush.nvim" },
		config = function()
			vim.g.zenbones_transparent_background = true
		end,
	},

	-- SpaceGray theme
	-- {
	-- 	"ajh17/Spacegray.vim",
	-- 	lazy = true,
	-- },

	-- SpaceGray theme  
	{
		"hhff/SpacegrayEighties.vim",
		lazy = true,
	},

	-- Spacedust theme
	{
		"marcelbeumer/spacedust.vim",
		lazy = true,
	},

	-- Palenight theme
	{
		"drewtempelmeyer/palenight.vim",
		lazy = true,
		config = function()
			vim.g.palenight_terminal_italics = 1
		end,
	},

	-- Base16 themes (includes rebecca, hopscotch, spacedust)
	{
		"chriskempson/base16-vim",
		lazy = true,
	},

}
