import gi
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk

try:
    Gtk.init()
    print("GTK initialized successfully!")
except Exception as e:
    print(f"Failed to initialize GTK: {e}")
