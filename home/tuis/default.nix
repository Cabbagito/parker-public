# The glance/control fleet: sys=btop, files=yazi, git=lazygit, audio=wiremix
# (+ fastfetch for the ritual), bluetooth=bluetui.
# music (spotify-player + cava) has its own file — see music.nix.
#
# btop/lazygit/yazi run through their HM modules (settings ported verbatim
# from the old conf files) so the stylix targets can inject colors — a
# hand-linked conf and a stylix-touched programs.* module would collide on
# the same file. fastfetch and gh-dash have no stylix target and keep plain
# files.
{ pkgs, ... }:
{
  imports = [ ./music.nix ];

  home.packages = with pkgs; [
    bluetui
    fastfetch
    gdu # stylix writes its gdu.yaml theme
    jq
    wiremix
  ];

  # Ported from btop.conf (dotfiles vintage); color_theme dropped — stylix
  # generates and selects its own theme. btop still grumbles about the
  # read-only config symlink on exit (AGENTS.md gotcha, cosmetic).
  programs.btop = {
    enable = true;
    settings = {
      theme_background = false;
      truecolor = true;
      force_tty = false;
      presets = "cpu:1:default,proc:0:default cpu:0:default,mem:0:default,net:0:default cpu:0:block,net:0:tty";
      vim_keys = true;
      rounded_corners = true;
      terminal_sync = true;
      graph_symbol = "braille";
      graph_symbol_cpu = "default";
      graph_symbol_mem = "default";
      graph_symbol_net = "default";
      graph_symbol_proc = "default";
      shown_boxes = "cpu mem net proc";
      update_ms = 500;
      proc_sorting = "memory";
      proc_reversed = false;
      proc_tree = false;
      proc_colors = true;
      proc_gradient = true;
      proc_per_core = false;
      proc_mem_bytes = true;
      proc_cpu_graphs = true;
      proc_info_smaps = false;
      proc_left = false;
      proc_filter_kernel = false;
      proc_aggregate = false;
      keep_dead_proc_usage = false;
      cpu_graph_upper = "Auto";
      cpu_graph_lower = "Auto";
      cpu_invert_lower = true;
      cpu_single_graph = false;
      cpu_bottom = false;
      show_uptime = true;
      show_cpu_watts = true;
      check_temp = true;
      cpu_sensor = "Auto";
      show_coretemp = true;
      cpu_core_map = "";
      temp_scale = "celsius";
      base_10_sizes = false;
      show_cpu_freq = true;
      clock_format = "%X";
      background_update = true;
      custom_cpu_name = "";
      disks_filter = "";
      mem_graphs = true;
      mem_below_net = false;
      zfs_arc_cached = true;
      show_swap = true;
      swap_disk = true;
      show_disks = true;
      only_physical = true;
      use_fstab = true;
      zfs_hide_datasets = false;
      disk_free_priv = false;
      show_io_stat = true;
      io_mode = false;
      io_graph_combined = false;
      io_graph_speeds = "";
      net_download = 100;
      net_upload = 100;
      net_auto = true;
      net_sync = true;
      net_iface = "";
      base_10_bitrate = "Auto";
      show_battery = true;
      selected_battery = "Auto";
      show_battery_watts = true;
      log_level = "WARNING";
      save_config_on_exit = true;
    };
  };

  # Ported from lazygit.yml — it held exactly this one key.
  programs.lazygit = {
    enable = true;
    settings.notARepository = "skip";
  };

  # Ported from yazi.toml. The DECISIONS.md opener set (imv images / mpv
  # video / Chrome PDFs) still lands with the media-stack step.
  programs.yazi = {
    enable = true;
    settings = {
      opener = {
        nvim = [
          {
            run = "nvim %s";
            block = true;
            desc = "Neovim";
          }
        ];
        open = [
          {
            run = "xdg-open %s";
            orphan = true;
            desc = "Open";
          }
        ];
      };
      # Rule matchers renamed upstream: name → url (current yazi errors on
      # `name`).
      open.prepend_rules = [
        {
          url = "*/";
          use = [ "open" ];
        }
        {
          mime = "text/*";
          use = [
            "nvim"
            "open"
          ];
        }
      ];
    };
  };

  xdg.configFile."fastfetch/config.jsonc".source = ./fastfetch.jsonc;
  xdg.configFile."fastfetch/logo.txt".source = ./logo.txt;
  xdg.configFile."gh-dash/config.yml".source = ./gh-dash.yml;
}
