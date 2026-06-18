import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';

// Define the API blueprint using standard D-Bus XML notation
const ExtensionInterfaceXML = `
<node>
  <interface name="com.clipsync.Extension">
    <method name="SetClipboard">
      <arg type="s" name="text" direction="in"/>
    </method>
  </interface>
</node>
`;

export default class ClipSyncExtension extends Extension {
    enable() {
        this._clipboard = St.Clipboard.get_default();
        this._selection = global.display.get_selection();
        
        this._selectionListenerId = this._selection.connect('owner-changed', (selection, type, selectionSource) => {
            if (type === 1) { 
                this._handleClipboardChange();
            }
        });

        // 1. Parse the XML string into a proper DBus Node Info object
        let nodeInfo = Gio.DBusNodeInfo.new_for_xml(ExtensionInterfaceXML);
        let interfaceInfo = nodeInfo.interfaces[0];

        // 2. Export the interface mapping directly onto the user's session bus
        this._dbusRegistrationId = Gio.DBus.session.register_object(
            '/com/clipsync/Extension',
            interfaceInfo,
            (connection, sender, objectPath, interfaceName, methodName, parameters, invocation) => {
                if (methodName === 'SetClipboard') {
                    let [text] = parameters.deep_unpack();
                    this._setSystemClipboard(text);
                    invocation.return_value(null); // Signal success back to Rust
                }
            },
	    null,
	    null
        );

        console.log('ClipSync Helper Extension Successfully Registered over D-Bus!');
    }

    disable() {
        if (this._selectionListenerId && this._selection) {
            this._selection.disconnect(this._selectionListenerId);
            this._selectionListenerId = null;
        }

        // Unregister from D-Bus when disabled to avoid trailing dead nodes
        if (this._dbusRegistrationId) {
            Gio.DBus.session.unregister_object(this._dbusRegistrationId);
            this._dbusRegistrationId = null;
        }

        this._selection = null;
        this._clipboard = null;
        console.log('ClipSync Helper Extension Disabled.');
    }

    _setSystemClipboard(text) {
        if (text && this._clipboard) {
            this._clipboard.set_text(St.ClipboardType.CLIPBOARD, text);
            console.log('ClipSync: Updated system clipboard from network incoming data!');
        }
    }

    _handleClipboardChange() {
        if (!this._clipboard) return;
        this._clipboard.get_text(St.ClipboardType.CLIPBOARD, (clipboard, text) => {
            if (text && text.trim() !== "") {
                this._sendToRustDaemon(text);
            }
        });
    }

    _sendToRustDaemon(text) {
        let bus = Gio.DBus.session;
        bus.call(
            'com.clipsync.Daemon',           
            '/com/clipsync/Daemon',          
            'com.clipsync.Daemon',          
            'UpdateClipboard',              
            new GLib.Variant('(s)', [text]), 
            null, Gio.DBusCallFlags.NONE, -1, null,
            (connection, res) => { try { bus.call_finish(res); } catch (e) {} }
        );
    }
}
