-- Treesitter configuration
-- Syntax highlighting and code understanding
return {
	{
		"nvim-treesitter/nvim-treesitter",
		-- Pinned: upstream's `main` rewrite drops nvim-treesitter.configs;
		-- this config targets the frozen legacy API on `master`.
		branch = "master",
		build = ":TSUpdate",
		event = { "BufReadPost", "BufNewFile" },

		config = function()
			require("nvim-treesitter.configs").setup({
				-- A list of parser names, or "all" (the listed parsers MUST always be installed)
				ensure_installed = {
					"c",
					"lua",
					"vim",
					"vimdoc",
					"query",
					"markdown",
					"markdown_inline",
					"python",
					"rust",
				},

				-- Automatically install missing parsers when entering buffer
				-- Recommendation: set to false if you don't have `tree-sitter` CLI installed locally
				auto_install = true,

				highlight = {
					enable = true,
					additional_vim_regex_highlighting = false,
				},

				indent = {
					enable = true,
				},
			})
		end,
	},
}
