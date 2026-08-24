GADGET_ROOT=$(find /config/usb_gadget/ -mindepth 1 -maxdepth 1 -type d | head -n 1)
CONFIG_ROOT=$(find "$GADGET_ROOT/configs/" -mindepth 1 -maxdepth 1 -type d | head -n 1)
UDC_NAME=$(cat "$GADGET_ROOT/UDC")
echo "" > "$GADGET_ROOT/UDC"
mkdir -p "$GADGET_ROOT/functions/mass_storage.0"
rm -f "$CONFIG_ROOT/mass_storage.0"
ln -s "$GADGET_ROOT/functions/mass_storage.0" "$CONFIG_ROOT"
echo "0" > "$GADGET_ROOT/functions/mass_storage.0/lun.0/cdrom"
echo "1" > "$GADGET_ROOT/functions/mass_storage.0/lun.0/ro"
echo "" > "$GADGET_ROOT/functions/mass_storage.0/lun.0/file"
echo "/tmp/android_deploy" > "$GADGET_ROOT/functions/mass_storage.0/lun.0/file"
echo "$UDC_NAME" > "$GADGET_ROOT/UDC"
