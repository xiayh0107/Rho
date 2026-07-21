test_that("execution retains workspace state", {
  workspace <- new.env(parent = baseenv())
  result <- rho_execute("x <- 41; x + 1", envir = workspace)

  expect_true(result$ok)
  expect_equal(workspace$x, 41)
  expect_match(result$value, "42")
})

test_that("errors and prior mutations are retained", {
  workspace <- new.env(parent = baseenv())
  result <- rho_execute("x <- 1; stop('boom')", envir = workspace)

  expect_false(result$ok)
  expect_equal(workspace$x, 1)
  expect_equal(result$error$message, "boom")
  expect_gt(length(result$calls), 0L)
})

test_that("object inspection is bounded metadata", {
  workspace <- new.env(parent = baseenv())
  workspace$x <- data.frame(a = 1:10, b = letters[1:10])
  result <- rho_inspect_object("x", envir = workspace)

  expect_true(result$ok)
  expect_equal(result$dimensions, c(10L, 2L))
  expect_true("data.frame" %in% result$classes)
  expect_equal(result$preview$kind, "tabular")
  expect_equal(length(result$preview$rows), 8L)
  expect_lt(nchar(result$structure), 4001L)
})

test_that("workspace snapshot reports environment contract", {
  workspace <- new.env(parent = baseenv())
  workspace$qc <- data.frame(sample = letters[1:4], value = 1:4)
  result <- rho_workspace_snapshot(envir = workspace, object_limit = 10L)

  expect_true(result$ok)
  expect_true(is.list(result$environment$renv))
  expect_true(is.list(result$environment$render))
  expect_true(any(vapply(result$objects, function(item) identical(item$name, "qc"), logical(1))))
})

test_that("scientific classes expose semantic bounded metadata without package loading", {
  fake_seurat <- structure(list(), class = "Seurat")
  fake_sce <- structure(list(), class = c("SingleCellExperiment", "SummarizedExperiment"))
  fake_se <- structure(list(), class = "SummarizedExperiment")
  fake_ranges <- structure(list(1L, 2L, 3L), class = "GRanges")
  fake_plot <- structure(
    list(
      data = data.frame(x = 1:3, y = 3:1),
      mapping = list(x = quote(x), y = quote(y)),
      layers = list(list()),
      labels = list(title = "bounded plot")
    ),
    class = c("ggplot", "gg")
  )

  expect_equal(rho_semantic_metadata(fake_seurat)$kind, "seurat")
  expect_equal(rho_semantic_metadata(fake_sce)$kind, "single_cell_experiment")
  expect_equal(rho_semantic_metadata(fake_se)$kind, "summarized_experiment")
  expect_equal(rho_semantic_metadata(fake_ranges)$kind, "genomic_ranges")
  plot_metadata <- rho_semantic_metadata(fake_plot)
  expect_equal(plot_metadata$kind, "ggplot")
  expect_equal(plot_metadata$layers, 1L)
  expect_equal(plot_metadata$data_dimensions, c(3L, 2L))
  expect_equal(rho_preview_kind(fake_plot), "plot")
})

test_that("vector previews stay bounded", {
  workspace <- new.env(parent = baseenv())
  workspace$x <- 1:100
  result <- rho_inspect_object("x", envir = workspace)

  expect_equal(result$preview$kind, "vector")
  expect_lte(length(result$preview$values), 12L)
  expect_true(result$preview$truncated)
})

test_that("tabular previews bound nested and long cell payloads by bytes", {
  workspace <- new.env(parent = baseenv())
  workspace$x <- data.frame(id = 1L)
  workspace$x$payload <- I(list(strrep("x", 1000000L)))
  result <- rho_inspect_object("x", envir = workspace)
  encoded <- jsonlite::toJSON(result, auto_unbox = TRUE, null = "null")

  expect_lt(nchar(encoded, type = "bytes"), 50000L)
  expect_match(result$preview$rows[[1L]]$payload, "truncated|length")
})

test_that("list previews bound long item names", {
  workspace <- new.env(parent = baseenv())
  workspace$x <- setNames(list(1L), strrep("x", 1000000L))
  result <- rho_inspect_object("x", envir = workspace)
  encoded <- jsonlite::toJSON(result, auto_unbox = TRUE, null = "null")

  expect_lt(nchar(encoded, type = "bytes"), 50000L)
  expect_match(result$preview$items[[1L]], "truncated")
})

test_that("render probe degrades cleanly when tooling is unavailable", {
  file <- tempfile(fileext = ".qmd")
  writeLines("---\ntitle: Test\n---\n\nHello", file)
  result <- rho_render_document(file)

  expect_true(is.list(result$capability))
  if (isTRUE(result$capability$can_render_qmd)) {
    expect_true(isTRUE(result$ok) || !is.null(result$error))
  } else {
    expect_false(result$ok)
    expect_equal(result$error$phase, "capability")
  }
})
