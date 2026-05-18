use zaroxi_interface_desktop::presenters::GpuShellPresenter;

#[test]
fn region_mapping_basic() {
    let width = 800;
    let height = 600;
    let chrome = 60;
    let status = 24;

    let regions = GpuShellPresenter::map_regions(width, height, chrome, status);

    // chrome at top
    assert_eq!(regions.chrome.x, 0);
    assert_eq!(regions.chrome.y, 0);
    assert_eq!(regions.chrome.width, width);
    assert_eq!(regions.chrome.height, chrome);

    // status at bottom
    assert_eq!(regions.status.x, 0);
    assert_eq!(regions.status.y, chrome + (height - chrome - status - 0u32));
    assert_eq!(regions.status.width, width);
    assert_eq!(regions.status.height, status);

    // content fills the middle
    assert_eq!(regions.content.x, 0);
    assert_eq!(regions.content.y, chrome);
    assert_eq!(regions.content.width, width);
    assert_eq!(regions.content.height, height - chrome - status);
}

#[test]
fn paint_buffer_size_mismatch_is_noop() {
    let width = 64;
    let height = 64;
    let chrome = 8;
    let status = 4;

    let regions = GpuShellPresenter::map_regions(width, height, chrome, status);
    let mut buf = vec![0u8; (width as usize) * (height as usize) * 4 - 4]; // intentionally wrong size
    // Should not panic; paint_to_buffer silently returns when size mismatches.
    GpuShellPresenter::paint_to_buffer(width, height, &mut buf, &regions);
}
