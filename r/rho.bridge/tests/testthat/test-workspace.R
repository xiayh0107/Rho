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
  encoded <- rho.bridge:::rho_json_encode(result)

  expect_lt(nchar(encoded, type = "bytes"), 50000L)
  expect_match(result$preview$rows[[1L]]$payload, "truncated|length")
})

test_that("list previews bound long item names", {
  workspace <- new.env(parent = baseenv())
  workspace$x <- setNames(list(1L), strrep("x", 1000000L))
  result <- rho_inspect_object("x", envir = workspace)
  encoded <- rho.bridge:::rho_json_encode(result)

  expect_lt(nchar(encoded, type = "bytes"), 50000L)
  expect_match(result$preview$items[[1L]], "truncated")
})

test_that("base R JSON encoding covers bridge scalar and vector values", {
  expect_identical(rho.bridge:::rho_json_encode(NULL), "null")
  expect_identical(rho.bridge:::rho_json_encode(TRUE), "true")
  expect_identical(rho.bridge:::rho_json_encode(c(TRUE, FALSE, NA)), "[true,false,null]")
  expect_identical(rho.bridge:::rho_json_encode(42L), "42")
  expect_identical(rho.bridge:::rho_json_encode(c(1L, NA_integer_)), "[1,null]")
  expect_identical(rho.bridge:::rho_json_encode(1.25), "1.25")
  expect_identical(
    rho.bridge:::rho_json_encode(c(NA_real_, NaN, Inf, -Inf)),
    "[null,null,null,null]"
  )
  expect_identical(rho.bridge:::rho_json_encode(character()), "[]")
  expect_identical(
    rho.bridge:::rho_json_encode(c("alpha", NA_character_, "omega")),
    '["alpha",null,"omega"]'
  )
})

test_that("base R JSON encoding escapes strings and preserves UTF-8", {
  expect_identical(
    rho.bridge:::rho_json_encode("\b\f\n\r\t\"\\"),
    '"\\b\\f\\n\\r\\t\\\"\\\\"'
  )
  expect_identical(rho.bridge:::rho_json_encode("\u0001"), '"\\u0001"')
  expect_identical(
    enc2utf8(rho.bridge:::rho_json_encode("雪😀")),
    enc2utf8('"雪😀"')
  )
})

test_that("base R JSON encoding distinguishes named and unnamed lists", {
  expect_identical(
    rho.bridge:::rho_json_encode(list(TRUE, c("x", "y"), NULL)),
    '[true,["x","y"],null]'
  )
  expect_identical(
    rho.bridge:::rho_json_encode(list(ok = TRUE, nested = list(value = 2L))),
    '{"ok":true,"nested":{"value":2}}'
  )
  expect_identical(rho.bridge:::rho_json_encode(setNames(list(), character())), "{}")
})

test_that("base R JSON encoding rejects unbounded or ambiguous object graphs", {
  expect_error(
    rho.bridge:::rho_json_encode(structure(list(value = 1L), class = "custom")),
    "classed object"
  )
  expect_error(rho.bridge:::rho_json_encode(globalenv()), "R type <environment>")
  expect_error(rho.bridge:::rho_json_encode(list(a = 1L, 2L)), "non-empty")
  expect_error(rho.bridge:::rho_json_encode(list(a = 1L, a = 2L)), "unique")

  nested <- 1L
  for (index in seq_len(4L)) {
    nested <- list(nested)
  }
  expect_error(rho.bridge:::rho_json_encode(nested, max_depth = 2L), "nesting limit")
  expect_error(rho.bridge:::rho_json_encode(list(1L, 2L), max_values = 2L), "value limit")
  expect_error(rho.bridge:::rho_json_encode(1:3, max_values = 2L), "value limit")
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
