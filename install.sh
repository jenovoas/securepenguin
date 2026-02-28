#!/bin/bash
# Script de instalación de SecurePenguin Inventory Scanner

set -e

echo "🐧 Instalando SecurePenguin Inventory Scanner..."

# Directorio de instalación
INSTALL_DIR="$HOME/Dev/securepenguin-inventory"
BIN_DIR="$INSTALL_DIR/target/release"
BINARY="$BIN_DIR/securepenguin-inventory"
LINK="$HOME/.local/bin/securepenguin-inventory"

# Compilar si es necesario
if [ ! -f "$BINARY" ]; then
    echo "📦 Compilando..."
    cd "$INSTALL_DIR"
    cargo build --release
fi

# Crear directorio de binarios si no existe
mkdir -p "$HOME/.local/bin"

# Crear symlink
echo "🔗 Creando symlink..."
ln -sf "$BINARY" "$LINK"

# Agregar alias a zshrc si no existe
ZSHRC="$HOME/.zshrc"
ALIAS_CMD="alias scan-inventory='$LINK'"

if ! grep -q "alias scan-inventory" "$ZSHRC" 2>/dev/null; then
    echo "📝 Agregando alias a ~/.zshrc..."
    echo "" >> "$ZSHRC"
    echo "# SecurePenguin Inventory Scanner" >> "$ZSHRC"
    echo "$ALIAS_CMD" >> "$ZSHRC"
    echo ""
    echo "✅ Alias 'scan-inventory' agregado a ~/.zshrc"
    echo "   Ejecuta: source ~/.zshrc"
else
    echo "✅ Alias 'scan-inventory' ya existe en ~/.zshrc"
fi

echo ""
echo "🎉 Instalación completa!"
echo ""
echo "📋 Uso:"
echo "   scan-inventory"
echo ""
echo "📄 Reporte se guarda en: ~/SecurePenguin/INVENTARIO_STATUS_AUTO.md"
echo ""
