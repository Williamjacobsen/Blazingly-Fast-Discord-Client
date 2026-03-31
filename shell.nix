# shell.nix
{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    rustc
    cargo
    rustfmt
    clippy

    pkg-config
    
    # --- FONT SUPPORT (Crucial for fixing the panic) ---
    fontconfig
    dejavu_fonts  # Provides a standard sans-serif font
    freefont_ttf  # Fallback fonts

    # Wayland + graphics
    wayland
    wayland-protocols
    libxkbcommon
    libGL
    vulkan-loader
    mesa

    # X11 fallback
    xorg.libX11
    xorg.libXcursor
    xorg.libXi
    xorg.libXrandr

    # OpenSSL
    openssl
  ];

  # This helps pkg-config and dynamic loading
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
    wayland
    libxkbcommon
    libGL
    vulkan-loader
    mesa
    openssl
    fontconfig # Add fontconfig to LD_LIBRARY_PATH
  ]);

  # Improved PKG_CONFIG_PATH to include multiple dependencies
  PKG_CONFIG_PATH = pkgs.lib.makeSearchPath "lib/pkgconfig" (with pkgs; [
    openssl.dev
    fontconfig.dev
  ]);

  shellHook = ''
    # 1. Point to the font configuration files
    export FONTCONFIG_FILE=${pkgs.fontconfig.out}/etc/fonts/fonts.conf
    
    # 2. Add the provided fonts to the fontconfig cache
    export FONTCONFIG_PATH=${pkgs.fontconfig.out}/etc/fonts
    
    # 3. Force XDG_DATA_DIRS so fontconfig can find the .ttf files
    export XDG_DATA_DIRS=$XDG_DATA_DIRS:${pkgs.dejavu_fonts}/share:${pkgs.freefont_ttf}/share
    
    echo "Discord Client Dev Shell: Fonts and Graphics initialized."
  '';
}
