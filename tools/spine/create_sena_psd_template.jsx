/*
 * Create the production-layer scaffold for Sena in Adobe Photoshop.
 *
 * The script:
 *   - reads pets/sena/spine/settings/layer_contract.json;
 *   - creates a 2400 x 3000 transparent RGB/8 document;
 *   - creates one group per required_groups entry;
 *   - creates every required named art layer;
 *   - saves as pets/sena/spine/source/sena.psd;
 *   - refuses to overwrite an existing sena.psd.
 *
 * It creates EMPTY art layers on purpose. The final Source Gate rejects empty
 * pixel bounds, so this is a drawing scaffold, not a fake passing asset.
 */
(function () {
    var CANVAS_WIDTH = 2400;
    var CANVAS_HEIGHT = 3000;
    var RESOLUTION = 144;
    var OUTPUT_NAME = "sena.psd";

    function readText(file) {
        file.encoding = "UTF8";
        if (!file.open("r")) {
            throw new Error("Failed to open " + file.fsName);
        }
        var text = file.read();
        file.close();
        return text;
    }

    function parseJson(text) {
        /*
         * ExtendScript builds bundled with older Photoshop versions may not
         * expose JSON.parse. The repository contract is trusted local data, so
         * eval is used only as a compatibility fallback.
         */
        if (typeof JSON !== "undefined" && JSON.parse) {
            return JSON.parse(text);
        }
        return eval("(" + text + ")");
    }

    function repoRoot() {
        if (!$.fileName) {
            throw new Error("Cannot resolve script path.");
        }

        var script = new File($.fileName);
        var spineTools = script.parent;
        var toolsDir = spineTools.parent;
        return toolsDir.parent;
    }

    function fileUnder(root, relativePath) {
        return new File(root.fsName + "/" + relativePath);
    }

    function folderUnder(root, relativePath) {
        return new Folder(root.fsName + "/" + relativePath);
    }

    function validateContract(contract) {
        if (!contract || contract.schema_version !== 1) {
            throw new Error("Unsupported layer contract schema.");
        }
        if (contract.character !== "sena") {
            throw new Error("Expected character sena.");
        }
        if (contract.naming !== "snake_case") {
            throw new Error("Expected snake_case layer naming.");
        }
        if (!contract.required_groups) {
            throw new Error("Layer contract has no required_groups.");
        }
    }

    function createGroupLayers(group, names) {
        /*
         * Photoshop inserts new ArtLayers at the top. Create in reverse order
         * so the visible Layers panel follows the contract order.
         */
        for (var i = names.length - 1; i >= 0; i--) {
            var layer = group.artLayers.add();
            layer.name = names[i];
            layer.opacity = 100;
            layer.visible = true;
        }
    }

    function savePsd(doc, outputFile) {
        var options = new PhotoshopSaveOptions();
        options.alphaChannels = true;
        options.annotations = true;
        options.embedColorProfile = true;
        options.layers = true;
        options.maximizeCompatibility = true;
        options.spotColors = true;

        doc.saveAs(outputFile, options, true, Extension.LOWERCASE);
    }

    try {
        var root = repoRoot();
        var contractFile = fileUnder(
            root,
            "pets/sena/spine/settings/layer_contract.json"
        );
        var outputDir = folderUnder(root, "pets/sena/spine/source");
        var outputFile = fileUnder(root, "pets/sena/spine/source/" + OUTPUT_NAME);

        if (!contractFile.exists) {
            throw new Error(
                "Layer contract not found:\n" + contractFile.fsName
            );
        }
        if (!outputDir.exists && !outputDir.create()) {
            throw new Error(
                "Failed to create source directory:\n" + outputDir.fsName
            );
        }
        if (outputFile.exists) {
            alert(
                "Sena PSD already exists.\n\n" +
                outputFile.fsName + "\n\n" +
                "Template generation refuses to overwrite production art."
            );
            return;
        }

        var contract = parseJson(readText(contractFile));
        validateContract(contract);

        var doc = app.documents.add(
            CANVAS_WIDTH,
            CANVAS_HEIGHT,
            RESOLUTION,
            "sena",
            NewDocumentMode.RGB,
            DocumentFill.TRANSPARENT
        );
        doc.bitsPerChannel = BitsPerChannelType.EIGHT;

        /*
         * Add groups in reverse because Photoshop inserts each new LayerSet at
         * the top of the document.
         */
        var groupNames = [];
        var key;
        for (key in contract.required_groups) {
            if (contract.required_groups.hasOwnProperty(key)) {
                groupNames.push(key);
            }
        }

        for (var i = groupNames.length - 1; i >= 0; i--) {
            var groupName = groupNames[i];
            var group = doc.layerSets.add();
            group.name = groupName;
            createGroupLayers(group, contract.required_groups[groupName]);
        }

        savePsd(doc, outputFile);

        var requiredCount = 0;
        for (key in contract.required_groups) {
            if (contract.required_groups.hasOwnProperty(key)) {
                requiredCount += contract.required_groups[key].length;
            }
        }

        alert(
            "Sena PSD template created.\n\n" +
            outputFile.fsName + "\n\n" +
            "Canvas: " + CANVAS_WIDTH + " x " + CANVAS_HEIGHT + "\n" +
            "Required art layers: " + requiredCount + "\n\n" +
            "All layers are intentionally empty. Paint real pixels before running the final Source Gate."
        );
    } catch (error) {
        alert(
            "Failed to create Sena PSD template.\n\n" +
            error.message
        );
    }
}());
