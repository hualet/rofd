import org.ofdrw.converter.ImageMaker;
import org.ofdrw.reader.OFDReader;

import javax.imageio.ImageIO;
import java.awt.image.BufferedImage;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.List;
import java.util.stream.Collectors;
import java.util.stream.Stream;

/**
 * Renders every migrated fixture to reference PNGs using ofdrw itself.
 *
 * <p>Not part of the Rust build; run via tools/render-references.sh after
 * building ofdrw with Maven. Output layout mirrors fixtures/:
 * references/&lt;module&gt;/&lt;name&gt;.ofd/&lt;page&gt;.png plus a
 * manifest.json recording per-file results.
 */
public class RenderReferences {

    /** 96 DPI in pixels per millimetre, matching OFD2IMGTest's 3.78 ppm. */
    private static final double PPM = 96 / 25.4;

    /** Cap per-document pages to bound repository size. */
    private static final int MAX_PAGES = 5;

    public static void main(String[] args) throws Exception {
        Path fixtures = Paths.get(args[0]).toAbsolutePath().normalize();
        Path references = Paths.get(args[1]).toAbsolutePath().normalize();

        List<Path> files;
        try (Stream<Path> walk = Files.walk(fixtures)) {
            files = walk.filter(p -> p.toString().endsWith(".ofd"))
                    .sorted()
                    .collect(Collectors.toList());
        }

        List<String> entries = new ArrayList<>();
        for (Path file : files) {
            String relative = fixtures.relativize(file).toString().replace('\\', '/');
            Path outDir = references.resolve(relative);
            int pageCount = 0;
            int rendered = 0;
            String error = null;
            try (OFDReader reader = new OFDReader(file)) {
                ImageMaker maker = new ImageMaker(reader, PPM);
                maker.config.setDrawBoundary(false);
                pageCount = maker.pageSize();
                rendered = Math.min(pageCount, MAX_PAGES);
                Files.createDirectories(outDir);
                for (int i = 0; i < rendered; i++) {
                    BufferedImage image = maker.makePage(i);
                    ImageIO.write(image, "PNG", outDir.resolve(i + ".png").toFile());
                }
                System.out.println("ok      " + relative + " (" + rendered + "/" + pageCount + " pages)");
            } catch (Throwable throwable) {
                error = throwable.toString();
                System.out.println("FAILED  " + relative + ": " + error);
            }
            entries.add(String.format(
                    "    {\"fixture\": \"%s\", \"pageCount\": %d, \"pagesRendered\": %d, \"status\": \"%s\"%s}",
                    escapeJson(relative), pageCount, rendered,
                    error == null ? "ok" : "error",
                    error == null ? "" : String.format(", \"error\": \"%s\"", escapeJson(error))));
        }

        String manifest = "{\n"
                + "  \"generator\": \"ofdrw ImageMaker (AWTMaker), drawBoundary=false\",\n"
                + "  \"dpi\": 96,\n"
                + "  \"maxPagesPerDocument\": " + MAX_PAGES + ",\n"
                + "  \"ofdrw\": \"https://github.com/ofdrw/ofdrw\",\n"
                + "  \"fixtures\": [\n"
                + String.join(",\n", entries)
                + "\n  ]\n}\n";
        Files.createDirectories(references);
        Files.write(references.resolve("manifest.json"), manifest.getBytes(StandardCharsets.UTF_8));
        System.out.println("manifest written to " + references.resolve("manifest.json"));
    }

    private static String escapeJson(String text) {
        StringBuilder out = new StringBuilder(text.length());
        for (char c : text.toCharArray()) {
            switch (c) {
                case '"': out.append("\\\""); break;
                case '\\': out.append("\\\\"); break;
                default:
                    if (c < 0x20) {
                        out.append(String.format("\\u%04x", (int) c));
                    } else {
                        out.append(c);
                    }
            }
        }
        return out.toString();
    }
}
