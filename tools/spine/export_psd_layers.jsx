/*
 * Sena PSD layer inventory exporter for Adobe Photoshop.
 *
 * Usage:
 *   1. Open pets/sena/spine/source/sena.psd in Photoshop.
 *   2. File -> Scripts -> Browse...
 *   3. Select this file.
 *   4. The script writes sena.layers.json next to the PSD.
 *
 * The script never edits the document.
 */
(function () {
    if (app.documents.length === 0) {
        alert("Open sena.psd before running this script.");
        return;
    }

    var doc = app.activeDocument;
    if (!doc.saved || !doc.fullName) {
        alert("Save sena.psd before exporting its layer inventory.");
        return;
    }

    function esc(value) {
        return value
            .replace(/\\/g, "\\\\")
            .replace(/"/g, "\\"")
            .replace(/\r/g, "\\r")
            .replace(/\n/g, "\\n")
            .replace(/\t/g, "\\t");
    }

    function numberValue(unitValue) {
        try {
            return unitValue.as("px");
        } catch (e) {
            return Number(unitValue);
        }
    }

    function boundsArray(layer) {
        try {
            var b = layer.bounds;
            return [
                numberValue(b[0]),
                numberValue(b[1]),
                numberValue(b[2]),
                numberValue(b[3])
            ];
        } catch (e) {
            return null;
        }
    }

    function layerKindName(layer) {
        try {
            if (layer.typename === "LayerSet") return "group";
            return String(layer.kind);
        } catch (e) {
            return "unknown";
        }
    }

    function collect(container, parentPath, output) {
        for (var i = 0; i < container.layers.length; i++) {
            var layer = container.layers[i];
            var path = parentPath ? parentPath + "/" + layer.name : layer.name;
            var bounds = boundsArray(layer);

            output.push({
                name: layer.name,
                path: path,
                type: layer.typename === "LayerSet" ? "group" : "layer",
                kind: layerKindName(layer),
                visible: !!layer.visible,
                opacity: Number(layer.opacity),
                bounds: bounds
            });

            if (layer.typename === "LayerSet") {
                collect(layer, path, output);
            }
        }
    }

    var layers = [];
    collect(doc, "", layers);

    var result = {
        schema_version: 1,
        source_file: doc.name,
        width: numberValue(doc.width),
        height: numberValue(doc.height),
        color_mode: String(doc.mode),
        bits_per_channel: String(doc.bitsPerChannel),
        layer_count: layers.length,
        layers: layers
    };

    function writeJsonValue(value, indent) {
        if (value === null) return "null";
        var t = typeof value;
        if (t === "string") return "\"" + esc(value) + "\"";
        if (t === "number") return isFinite(value) ? String(value) : "null";
        if (t === "boolean") return value ? "true" : "false";

        var nextIndent = indent + "  ";
        var parts = [];
        var i;

        if (value instanceof Array) {
            for (i = 0; i < value.length; i++) {
                parts.push(nextIndent + writeJsonValue(value[i], nextIndent));
            }
            return "[\n" + parts.join(",\n") + "\n" + indent + "]";
        }

        for (var key in value) {
            if (value.hasOwnProperty(key)) {
                parts.push(
                    nextIndent + "\"" + esc(key) + "\": " +
                    writeJsonValue(value[key], nextIndent)
                );
            }
        }
        return "{\n" + parts.join(",\n") + "\n" + indent + "}";
    }

    var outputName = doc.name.replace(/\.[^\.]+$/, "") + ".layers.json";
    var outputFile = new File(doc.path + "/" + outputName);
    outputFile.encoding = "UTF8";
    if (!outputFile.open("w")) {
        alert("Failed to create " + outputFile.fsName);
        return;
    }

    outputFile.write(writeJsonValue(result, ""));
    outputFile.close();

    alert(
        "Sena layer inventory exported.\n\n" +
        outputFile.fsName + "\n\n" +
        "Layers: " + layers.length
    );
}());
