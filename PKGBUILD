pkgname=majusb-bootable-creator
pkgver=0.1.0
pkgrel=1
arch=('x86_64')
url="https://github.com/vicrodh/usb-bootable-creator"
license=('MIT')
depends=('gtk4' 'glib2' 'gio' 'polkit' 'coreutils' 'util-linux' 'dosfstools' 'ntfs-3g' 'parted' 'rsync')
makedepends=('rust' 'cargo')
source=("$pkgname-$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
  cd "$srcdir/$pkgname-$pkgver"
  cargo build --release
}

package() {
  cd "$srcdir/$pkgname-$pkgver"
  install -Dm755 target/release/rust-usb-bootable-creator "$pkgdir/usr/bin/rust-usb-bootable-creator"
  install -Dm755 target/release/cli_helper "$pkgdir/usr/bin/cli_helper"
  install -Dm644 assets/icons/icon-128x128.png "$pkgdir/usr/share/icons/hicolor/128x128/apps/majusb-bootable-creator.png"
  install -Dm644 majusb-bootable-creator.desktop "$pkgdir/usr/share/applications/majusb-bootable-creator.desktop"
}
