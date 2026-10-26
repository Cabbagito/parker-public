# SearXNG — parker's own web search. A metasearch engine on localhost that
# aggregates the public engines (DuckDuckGo, Google, Brave, …), so parker
# searches the web with no API key, no per-query bill, and no provider that
# can change its terms. The JSON API (`format=json`) is what `parker search`
# and the search MCP tool consume; the HTML UI on the same port is a freebie
# for the browser.
#
# Localhost only, same posture as the engine (127.0.0.1:4096) and
# coolercontrold (11987). The secret_key only signs image-proxy URLs and
# cookies for a service that never leaves the box, so it is minted once on
# the machine rather than kept anywhere: the engine password's pattern
# (engine.nix), rotation is `rm` + rebuild. The module runs envsubst over
# settings.yml at start, which is how `$SEARX_SECRET_KEY` gets in.
#
# Known weather: public engines rate-limit scrapers, and Google in
# particular blocks sustained heavy use. Metasearch degrades gracefully —
# results keep coming from the engines that still answer — and at
# one-person-plus-one-parker volume this is a non-issue until it visibly
# isn't (QUIRKS.md material if it ever happens).
{ pkgs, ... }:
{
  # The NixOS module still carries the ancestral name `services.searx`; the
  # package it runs is searxng. The file is named for the concern.
  services.searx = {
    enable = true;
    package = pkgs.searxng;
    environmentFile = "/var/lib/searx/secret.env";
    settings = {
      server = {
        bind_address = "127.0.0.1";
        port = 8888;
        secret_key = "$SEARX_SECRET_KEY";
        # Bot protection defends a public instance; this one answers only
        # its own machine, and the limiter needs redis we otherwise don't run.
        limiter = false;
      };
      # "json" is the whole reason this service exists — the default is
      # html-only and the API 403s without it.
      search.formats = [
        "html"
        "json"
      ];
    };
  };

  # Activation runs before systemd (re)starts units, at boot and on switch,
  # so the file is there before searx-init reads it.
  system.activationScripts.searxSecret = ''
    if [ ! -s /var/lib/searx/secret.env ]; then
      mkdir -p -m 700 /var/lib/searx
      (umask 077; printf 'SEARX_SECRET_KEY=%s\n' \
        "$(${pkgs.openssl}/bin/openssl rand -hex 32)" > /var/lib/searx/secret.env)
    fi
  '';
}
