{
  description = "Neote development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
            cmake
            clang
            lld
            tree-sitter
            llvmPackages.libclang
          ];


          buildInputs = with pkgs; [
            # Rust toolchain
            rustc
            cargo
            rustfmt
            clippy

            # System libraries
            libxkbcommon
            fontconfig
            freetype
            expat
            libglvnd
            libX11
            libXcursor
            libXi
            libXrandr
            vulkan-loader
            wayland

            # For workspace-daemon file operations
            openssl
            
            # D-Bus for xdg-desktop-portal (RFD xdg-portal feature)
            dbus

            # xdg-desktop-portal for RFD file dialogs on Wayland
            # portal-hyprland: compositor-specific portals (screenshot, etc.)
            # portal-gtk: REQUIRED for file chooser — portal-hyprland alone
            #   does not implement org.freedesktop.impl.portal.FileChooser
            glib
            gtk3
            pango
            atk
            gdk-pixbuf
            xdg-desktop-portal
            xdg-desktop-portal-hyprland
            xdg-desktop-portal-gtk
            gsettings-desktop-schemas  # For GTK3 settings

            # CLI picker fallback tools (zenity is the primary fallback)
            zenity
            kdePackages.kdialog
          ];

          # Environment variables
          env = {
            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
            # Use Wayland backend for GTK3 in Hyprland
            GDK_BACKEND = "wayland";
            # GTK3 theme settings for Nix environment
            GTK_THEME = "Adwaita";
            GTK_DATA_PREFIX = "${pkgs.gtk3}";
            # Ensure GTK can find its modules
            GTK_PATH = "${pkgs.gtk3}/lib/gtk-3.0:${pkgs.gtk3}/lib/gtk-3.0/3.0.0";
            # XDG_DATA_DIRS must include portal backends so xdg-desktop-portal
            # can discover them (via org.freedesktop.impl.portal.FileChooser etc.)
            XDG_DATA_DIRS = with pkgs; builtins.concatStringsSep ":" [
              "${gtk3}/share"
              "${gsettings-desktop-schemas}/share/gsettings-schemas/${gsettings-desktop-schemas.name}"
              "${xdg-desktop-portal-gtk}/share"
              "${xdg-desktop-portal-hyprland}/share"
            ];
            GI_TYPELIB_PATH = "${pkgs.gtk3}/lib/girepository-1.0";
            # Ensure pkg-config can find .pc files
            PKG_CONFIG_PATH = with pkgs; lib.makeSearchPathOutput "dev" "lib/pkgconfig" [
              webkitgtk_4_1
              glib
              gtk3
              pango
              atk
              gdk-pixbuf
              dbus
              libxkbcommon
              fontconfig
              freetype
              expat
              libglvnd
              libX11
              libXcursor
              libXi
              libXrandr
              vulkan-loader
              wayland
              openssl
              xdg-desktop-portal-hyprland
              xdg-desktop-portal-gtk
            ];
            # Ensure linker can find libraries
            LD_LIBRARY_PATH = with pkgs; lib.makeLibraryPath [
              libxkbcommon
              fontconfig
              freetype
              expat
              libglvnd
              libX11
              libXcursor
              libXi
              libXrandr
              vulkan-loader
              wayland
              openssl
              # GTK3 dependencies for RFD
              glib
              gtk3
              pango
              atk
              gdk-pixbuf
              # D-Bus may still be needed
              dbus
              # Portal backends
              xdg-desktop-portal-hyprland
              xdg-desktop-portal-gtk
              # Tree-sitter
              tree-sitter
              # WebKitGTK (optional webview dependencies)
              webkitgtk_4_1
            ];
          };

          shellHook = ''
            echo "Zaroxi development environment"
            echo "Run: cargo run --bin desktop"
            echo ""
            echo "Picker fallback tools:"
            echo "  zenity:	$(command -v zenity 2>/dev/null || echo '(missing)')"
            echo "  kdialog:	$(command -v kdialog 2>/dev/null || echo '(missing)')"
            echo "Portal backends:"
            echo "  portal-gtk:	$(command -v xdg-desktop-portal-gtk 2>/dev/null || echo '(missing)')"
            echo "  portal-hyprland:	$(command -v xdg-desktop-portal-hyprland 2>/dev/null || echo '(missing)')"
          '';
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "zaroxi";
          version = "0.2.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            cmake
            # For GLib/GTK3
            wrapGAppsHook
          ];

          buildInputs = with pkgs; [
            libxkbcommon
            fontconfig
            freetype
            expat
            libglvnd
            libX11
            libXcursor
            libXi
            libXrandr
            vulkan-loader
            wayland
            openssl
            # GTK3 dependencies for RFD
            glib
            gtk3
            pango
            atk
            gdk-pixbuf
            gsettings-desktop-schemas  # For GTK3 settings
            # D-Bus may still be needed
            dbus
            # Portal backends
            xdg-desktop-portal-hyprland
            xdg-desktop-portal-gtk
            # CLI fallback tools
            zenity
            kdePackages.kdialog
            # Tree-sitter
            tree-sitter
            # WebKitGTK (optional webview dependencies)
            webkitgtk_4_1
          ];

        };
      }
    );
}
