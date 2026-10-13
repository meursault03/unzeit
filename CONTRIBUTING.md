# Contributing to Unzeit

[English](#english) · [Português](#português)

## English

Requires Node.js 22.12+, Rust 1.90+, CMake, Clang/libclang, GTK 3, and WebKitGTK 4.1.
See the [official Tauri Linux prerequisites](https://v2.tauri.app/start/prerequisites/#linux).
Also install FFmpeg/FFprobe and GStreamer codecs for your videos.

```sh
git clone https://github.com/meursault03/unzeit.git
cd unzeit
npm ci
npm run tauri dev
```

`npm run dev` is a browser preview; local files require the desktop app.
Run `npm run tauri build` to generate an AppImage in
`src-tauri/target/release/bundle/appimage/`.

### Linux packages

The recipes in `packaging/` require Python 3.11+ and use the project metadata
version. Run them from the repository root. Output goes to `artifacts/`; use
`--output` with a new directory to repeat staging. Courses, models, and databases
are excluded. No package has been published to AUR or Flathub.

<details>
<summary>Arch: build and install the local package</summary>

```sh
python3 packaging/prepare.py arch
cd artifacts/arch
makepkg -si
```

The 0.4.0 package was built and installed in an isolated Arch root; package
integrity and linked libraries passed verification. Also install your desktop's
portal backend. There is no AUR entry.

</details>

<details>
<summary>Fedora: build an RPM with Podman</summary>

```sh
python3 packaging/prepare.py fedora
podman build -t unzeit-rpm -f artifacts/fedora/Containerfile artifacts/fedora/source
mkdir -p artifacts/rpm
podman run --rm -v "$PWD/artifacts/rpm:/out:Z" unzeit-rpm
sudo dnf install artifacts/rpm/*.rpm
```

The recipe uses Fedora 43. Build on Fedora to preserve library compatibility.
RPM 0.4.0 was built and installed in an isolated Fedora 43 environment. Package
integrity, linked libraries, H.264/AAC playback and 16 kHz mono WAV extraction
passed verification. The interface and picker in a Fedora desktop session
still need testing.
Fedora's [free FFmpeg package](https://packages.fedoraproject.org/pkgs/ffmpeg/ffmpeg-free/)
supports fewer codecs; check playback and extraction with your files.

</details>

<details>
<summary>Flatpak: build the local recipe</summary>

Install `flatpak-builder` and the runtimes below. Staging downloads locked
dependencies for the SDK's offline build; add `--offline` if they are cached.

```sh
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user flathub org.gnome.Platform//50 org.gnome.Sdk//50 \
  org.freedesktop.Sdk.Extension.node22//25.08 \
  org.freedesktop.Sdk.Extension.rust-stable//25.08 \
  org.freedesktop.Sdk.Extension.llvm21//25.08
python3 packaging/prepare.py flatpak
flatpak-builder --user --install --force-clean --state-dir=artifacts/flatpak-state \
  artifacts/flatpak-build artifacts/flatpak/local.unzeit.desktop.json
flatpak run local.unzeit.desktop
```

Version 0.4.0 was built in the SDK and installed in an isolated environment.
Importing portal-authorized folders, H.264/AAC playback, seeking, playback speed,
WebVTT captions, and saving on close passed testing. The package uses runtime
FFmpeg/codecs and data separate from native installations, without general
access to your home folder. Manual interaction with the picker on your desktop
still needs checking.

</details>

### Checks

For performance changes, run the manual database benchmarks separately:

```sh
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --lib \
  benchmark -- --ignored --nocapture
```

```sh
npm run check
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --lib
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

`bash tests/webview-smoke.sh` checks native WebKit with generated media and
isolated app data. Reports and screenshots go to a temporary directory. Build
normally afterward; the `webview-smoke` feature is for testing only.

Use `UNZEIT_TEST_CLOSE_ONLY=1` to check shutdown and final progress,
`UNZEIT_TEST_PLAYING_CLOSE=1` to close during playback, or `UNZEIT_TEST_UX_ONLY=1`
for error and recovery flows. The real picker requires Python GI/AT-SPI and
an accessible portal window; `UNZEIT_TEST_PICKER_MOCK=1` checks app state with
controlled IPC when the Wayland portal window is inaccessible.

`GDK_BACKEND=x11 UNZEIT_TEST_SELECT_CLOSE=1 bash tests/webview-smoke.sh`
opens the language list and clicks the actual titlebar close button. This check
requires `xdotool` and `xprop`; it uses an isolated window and library.

To check real inference, provide a medium model and the
[official JFK sample](https://github.com/ggml-org/whisper.cpp/blob/master/samples/jfk.wav):

```sh
cargo build --manifest-path src-tauri/Cargo.toml --bin unzeit
UNZEIT_TEST_MODEL=/absolute/path/ggml-medium-q5_0.bin \
UNZEIT_TEST_AUDIO=/absolute/path/jfk.wav \
UNZEIT_TEST_EXECUTABLE="$PWD/src-tauri/target/debug/unzeit" \
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --lib \
  real_medium_transcription -- --ignored
```

## Português

Requer Node.js 22.12+, Rust 1.90+, CMake, Clang/libclang, GTK 3 e WebKitGTK 4.1.
Veja os [pré-requisitos oficiais do Tauri para Linux](https://v2.tauri.app/start/prerequisites/#linux).
Instale também FFmpeg/FFprobe e os codecs GStreamer dos seus vídeos.

```sh
git clone https://github.com/meursault03/unzeit.git
cd unzeit
npm ci
npm run tauri dev
```

`npm run dev` abre apenas a prévia no navegador; arquivos locais exigem o app
desktop. Para gerar um AppImage, execute `npm run tauri build`. O resultado fica
em `src-tauri/target/release/bundle/appimage/`.

### Pacotes para Linux

As receitas em `packaging/` usam Python 3.11+ e a versão dos metadados do projeto.
Execute na raiz do repositório. Os resultados ficam em `artifacts/`; para repetir
a preparação, use `--output` com uma pasta nova. Cursos, modelos e bancos não
entram no pacote. Nenhum pacote foi publicado no AUR ou no Flathub.

<details>
<summary>Arch: gerar e instalar o pacote local</summary>

```sh
python3 packaging/prepare.py arch
cd artifacts/arch
makepkg -si
```

O pacote 0.4.0 foi compilado e instalado em uma raiz Arch isolada; a verificação
de integridade e de bibliotecas passou. A receita declara as dependências de mídia;
instale também o backend do portal para seu desktop. Não há entrada no AUR.

</details>

<details>
<summary>Fedora: gerar o RPM com Podman</summary>

```sh
python3 packaging/prepare.py fedora
podman build -t unzeit-rpm -f artifacts/fedora/Containerfile artifacts/fedora/source
mkdir -p artifacts/rpm
podman run --rm -v "$PWD/artifacts/rpm:/out:Z" unzeit-rpm
sudo dnf install artifacts/rpm/*.rpm
```

A receita usa Fedora 43. Compile no Fedora para manter a compatibilidade de
bibliotecas. O RPM 0.4.0 foi compilado e instalado em um ambiente Fedora 43
isolado, com integridade, bibliotecas, reprodução H.264/AAC e extração WAV
mono a 16 kHz verificadas. A interface e o seletor em uma sessão Fedora
ainda precisam de teste.
O [FFmpeg livre do Fedora](https://packages.fedoraproject.org/pkgs/ffmpeg/ffmpeg-free/)
suporta menos codecs; confira reprodução e extração com seus arquivos.

</details>

<details>
<summary>Flatpak: compilar a receita local</summary>

Instale `flatpak-builder` e os runtimes abaixo. A preparação baixa dependências
fixadas para o build sem rede dentro do SDK; use `--offline` se já estiverem no cache.

```sh
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user flathub org.gnome.Platform//50 org.gnome.Sdk//50 \
  org.freedesktop.Sdk.Extension.node22//25.08 \
  org.freedesktop.Sdk.Extension.rust-stable//25.08 \
  org.freedesktop.Sdk.Extension.llvm21//25.08
python3 packaging/prepare.py flatpak
flatpak-builder --user --install --force-clean --state-dir=artifacts/flatpak-state \
  artifacts/flatpak-build artifacts/flatpak/local.unzeit.desktop.json
flatpak run local.unzeit.desktop
```

O build 0.4.0 foi compilado no SDK e instalado em um ambiente isolado. Importação
de pastas autorizadas pelo portal, reprodução H.264/AAC, avanço, velocidade,
legendas WebVTT e salvamento ao fechar passaram nos testes. O pacote usa
codecs/FFmpeg do runtime e dados separados da instalação nativa, sem acesso geral
à pasta pessoal. A interação manual com o seletor no seu desktop ainda precisa
ser conferida.

</details>

### Verificações

Para mudanças de desempenho, execute separadamente os benchmarks do banco:

```sh
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --lib \
  benchmark -- --ignored --nocapture
```

```sh
npm run check
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --lib
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

`bash tests/webview-smoke.sh` verifica o WebKit nativo com mídia gerada e dados
isolados. Guarda relatórios e imagens em uma pasta temporária. Compile normalmente
depois; a feature `webview-smoke` é exclusiva de teste.

Use `UNZEIT_TEST_CLOSE_ONLY=1` para verificar fechamento e posição final,
`UNZEIT_TEST_PLAYING_CLOSE=1` para fechar durante reprodução, ou
`UNZEIT_TEST_UX_ONLY=1` para os fluxos de erro e recuperação. O seletor real exige
Python GI/AT-SPI e um portal acessível; `UNZEIT_TEST_PICKER_MOCK=1` verifica o
estado do app com IPC controlado quando o portal Wayland não é acessível.

`GDK_BACKEND=x11 UNZEIT_TEST_SELECT_CLOSE=1 bash tests/webview-smoke.sh`
abre a lista de idiomas e clica no botão de fechar da barra de título. O teste
exige `xdotool` e `xprop` e usa uma janela e uma biblioteca isoladas.

Para verificar a inferência real, forneça o modelo medium e o
[áudio JFK oficial](https://github.com/ggml-org/whisper.cpp/blob/master/samples/jfk.wav):

```sh
cargo build --manifest-path src-tauri/Cargo.toml --bin unzeit
UNZEIT_TEST_MODEL=/caminho/ggml-medium-q5_0.bin \
UNZEIT_TEST_AUDIO=/caminho/jfk.wav \
UNZEIT_TEST_EXECUTABLE="$PWD/src-tauri/target/debug/unzeit" \
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --lib \
  real_medium_transcription -- --ignored
```
