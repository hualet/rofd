#include <stdio.h>
#include <string.h>

#include "rofd.h"

#define CHECK(condition) do { \
    if (!(condition)) { \
        fprintf(stderr, "navigation check failed at %s:%d: %s\n", __FILE__, __LINE__, #condition); \
        goto cleanup; \
    } \
} while (0)

int main(int argc, char **argv) {
    rofd_document_t *document = NULL;
    rofd_outline_t *outline = NULL;
    rofd_page_t *page = NULL;
    rofd_link_list_t *links = NULL;
    rofd_error_t *error = NULL;
    rofd_outline_node_t node = {0};
    rofd_action_t action = {0};
    struct {
        rofd_destination_t value;
        unsigned char tail[17];
    } destination;
    size_t count = 0;
    int result = 1;

    CHECK(argc == 2);
    CHECK(rofd_document_open(argv[1], NULL, &document, &error) == ROFD_STATUS_OK);
    CHECK(rofd_document_get_outline(document, &outline, &error) == ROFD_STATUS_OK);
    CHECK(outline != NULL);
    CHECK(rofd_document_get_page(document, 0, &page, &error) == ROFD_STATUS_OK);
    CHECK(rofd_page_get_links(page, &links, &error) == ROFD_STATUS_OK);
    CHECK(links != NULL);
    rofd_page_free(page);
    page = NULL;
    rofd_document_free(document);
    document = NULL;
    CHECK(rofd_outline_get_count(outline, &count, &error) == ROFD_STATUS_OK);
    CHECK(count == 3);
    node.struct_size = sizeof(node);
    CHECK(rofd_outline_get_node(outline, 0, &node, &error) == ROFD_STATUS_OK);
    CHECK(node.title != NULL && strcmp(node.title, "目录") == 0);
    CHECK(node.parent == ROFD_NO_INDEX && node.first_child == 1 && node.next_sibling == 2);
    CHECK(node.expanded == 0 && node.action_count == 0);
    CHECK(rofd_outline_get_node(outline, 1, &node, &error) == ROFD_STATUS_OK);
    CHECK(node.title != NULL && strcmp(node.title, "第二页") == 0);
    CHECK(node.parent == 0 && node.first_child == ROFD_NO_INDEX && node.next_sibling == ROFD_NO_INDEX);
    CHECK(node.expanded == 1 && node.action_count == 1);
    action.struct_size = sizeof(action);
    CHECK(rofd_outline_get_action(outline, 1, 0, &action, &error) == ROFD_STATUS_OK);
    CHECK(action.kind == ROFD_ACTION_GOTO && action.event == ROFD_ACTION_EVENT_CLICK);
    CHECK(action.type_name != NULL && strcmp(action.type_name, "Goto") == 0);
    CHECK(action.event_name != NULL && strcmp(action.event_name, "CLICK") == 0);
    memset(&destination, 0xa5, sizeof(destination));
    destination.value.struct_size = sizeof(destination);
    CHECK(rofd_outline_get_action_destination(outline, 1, 0, &destination.value, &error) == ROFD_STATUS_OK);
    CHECK(destination.value.struct_size == sizeof(destination));
    for (size_t i = 0; i < sizeof(destination.tail); ++i) CHECK(destination.tail[i] == 0xa5);
    CHECK(destination.value.kind == ROFD_DESTINATION_XYZ);
    CHECK(destination.value.flags == (ROFD_DESTINATION_HAS_PAGE_INDEX | ROFD_DESTINATION_HAS_PAGE_ID |
                                      ROFD_DESTINATION_HAS_LEFT | ROFD_DESTINATION_HAS_TOP));
    CHECK(destination.value.page_index == 1 && destination.value.page_id == 42);
    CHECK(destination.value.mode_name != NULL && strcmp(destination.value.mode_name, "XYZ") == 0);
    CHECK(destination.value.left_mm == 12.5 && destination.value.top_mm == 24.0);
    CHECK(destination.value.zoom == 0.0);
    CHECK(rofd_outline_get_action(outline, 2, 0, &action, &error) == ROFD_STATUS_OK);
    CHECK(action.kind == ROFD_ACTION_URI && action.uri != NULL);
    CHECK(strcmp(action.uri, "https://example.invalid/") == 0 && action.uri_base == NULL);
    CHECK(rofd_outline_get_action_destination(outline, 2, 0, &destination.value, &error) == ROFD_STATUS_UNSUPPORTED);
    CHECK(error != NULL && rofd_error_get_status(error) == ROFD_STATUS_UNSUPPORTED);
    CHECK(destination.value.flags == 0 && destination.value.mode_name == NULL);
    rofd_error_free(error);
    error = NULL;
    CHECK(rofd_outline_get_action(outline, 2, 1, &action, &error) == ROFD_STATUS_OK);
    CHECK(action.kind == ROFD_ACTION_ATTACHMENT && action.flags == ROFD_ACTION_NEW_WINDOW);
    CHECK(action.attachment_id != NULL && strcmp(action.attachment_id, "attached-file") == 0);
    CHECK(rofd_outline_get_action(outline, 2, 2, &action, &error) == ROFD_STATUS_OK);
    CHECK(action.kind == ROFD_ACTION_UNKNOWN && action.type_name != NULL);
    CHECK(strcmp(action.type_name, "Movie") == 0);
    CHECK(rofd_outline_get_node(outline, count, &node, &error) == ROFD_STATUS_PAGE_OUT_OF_RANGE);
    CHECK(node.title == NULL && node.action_count == 0 && node.struct_size == sizeof(node));
    rofd_error_free(error);
    error = NULL;
    CHECK(rofd_link_list_get_count(links, &count, &error) == ROFD_STATUS_OK && count == 5);
    CHECK(rofd_link_list_get_region_count(links, 1, &count, &error) == ROFD_STATUS_OK && count == 2);
    rofd_rect_t region = {0};
    CHECK(rofd_link_list_get_region(links, 1, 0, &region, &error) == ROFD_STATUS_OK);
    CHECK(region.x_mm == 1 && region.y_mm == 2 && region.width_mm == 3 && region.height_mm == 4);
    CHECK(rofd_link_list_get_region(links, 1, 1, &region, &error) == ROFD_STATUS_OK);
    CHECK(region.x_mm == 10 && region.y_mm == 20 && region.width_mm == 5 && region.height_mm == 8);
    CHECK(rofd_link_list_get_region(links, 4, 0, &region, &error) == ROFD_STATUS_OK);
    CHECK(region.x_mm == 10 && region.y_mm == 20 && region.width_mm == 20 && region.height_mm == 10);
    CHECK(rofd_link_list_get_action_count(links, 0, &count, &error) == ROFD_STATUS_OK && count == 1);
    CHECK(rofd_link_list_get_action(links, 0, 0, &action, &error) == ROFD_STATUS_OK);
    CHECK(action.kind == ROFD_ACTION_URI && action.event == ROFD_ACTION_EVENT_PAGE_OPEN);
    CHECK(action.uri != NULL && strcmp(action.uri, "file:///not-opened") == 0);
    CHECK(rofd_link_list_get_action_destination(links, 1, 0, &destination.value, &error) == ROFD_STATUS_OK);
    CHECK(destination.value.page_index == 1 && destination.value.page_id == 42);
    CHECK(destination.value.left_mm == 12.5 && destination.value.top_mm == 24.0 && destination.value.zoom == 0.0);
    CHECK((destination.value.flags & ROFD_DESTINATION_HAS_ZOOM) != 0);
    CHECK(destination.value.struct_size == sizeof(destination));
    for (size_t i = 0; i < sizeof(destination.tail); ++i) CHECK(destination.tail[i] == 0xa5);
    CHECK(rofd_link_list_get_action(links, 2, 0, &action, &error) == ROFD_STATUS_OK);
    CHECK(action.kind == ROFD_ACTION_ATTACHMENT && action.flags == 0);
    CHECK(action.attachment_id != NULL && strcmp(action.attachment_id, "attached-file") == 0);
    CHECK(rofd_link_list_get_action(links, 3, 0, &action, &error) == ROFD_STATUS_OK);
    CHECK(action.kind == ROFD_ACTION_UNKNOWN && action.event == ROFD_ACTION_EVENT_UNKNOWN);
    CHECK(action.type_name != NULL && strcmp(action.type_name, "Movie") == 0);
    CHECK(rofd_link_list_get_region(links, 0, SIZE_MAX, &region, &error) == ROFD_STATUS_PAGE_OUT_OF_RANGE);
    CHECK(region.x_mm == 0 && region.y_mm == 0 && region.width_mm == 0 && region.height_mm == 0);
    result = 0;

cleanup:
    if (result != 0 && error != NULL) fprintf(stderr, "%s\n", rofd_error_get_message(error));
    rofd_error_free(error);
    rofd_outline_free(outline);
    rofd_link_list_free(links);
    rofd_page_free(page);
    rofd_document_free(document);
    return result;
}
