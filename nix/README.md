# Flake install

This flake packages the **browser search CLI** (`kovcheg`). Full downloads require the [Torlink-Gui desktop app](https://github.com/Domanffe/kovcheg/releases).

```nix
inputs = {
  kovcheg-gui.url = "github:Domanffe/kovcheg";
};
```

**User (`home.nix`)**

```nix
{ pkgs, inputs, ... }:
{
  home.packages = [
    inputs.kovcheg-gui.packages.${pkgs.system}.default
  ];
}
```

**System (`configuration.nix`)**

```nix
{ pkgs, inputs, ... }:
{
  environment.systemPackages = [
    inputs.kovcheg-gui.packages.${pkgs.system}.default
  ];
}
```

After bumping `version` in `nix/package.nix`, refresh `src` and `npmDepsHash` with `nix-prefetch-url` / `nix-build`.
