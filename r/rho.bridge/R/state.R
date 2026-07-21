.rho_bridge_state <- new.env(parent = emptyenv())
.rho_bridge_state$last_execution <- NULL

json_escape_string <- function(value) {
  converted <- iconv(value, from = "", to = "UTF-8", sub = NA_character_)
  if (is.na(converted)) {
    stop("Cannot encode a string that is not valid UTF-8.", call. = FALSE)
  }

  codepoints <- utf8ToInt(converted)
  escaped <- vapply(codepoints, function(codepoint) {
    if (identical(codepoint, 0x22L)) {
      return("\\\"")
    }
    if (identical(codepoint, 0x5cL)) {
      return("\\\\")
    }
    if (identical(codepoint, 0x08L)) {
      return("\\b")
    }
    if (identical(codepoint, 0x0cL)) {
      return("\\f")
    }
    if (identical(codepoint, 0x0aL)) {
      return("\\n")
    }
    if (identical(codepoint, 0x0dL)) {
      return("\\r")
    }
    if (identical(codepoint, 0x09L)) {
      return("\\t")
    }
    if (codepoint < 0x20L) {
      return(sprintf("\\u%04x", codepoint))
    }
    intToUtf8(codepoint)
  }, character(1), USE.NAMES = FALSE)

  enc2utf8(paste0('"', paste0(escaped, collapse = ""), '"'))
}

json_number <- function(value) {
  if (!is.finite(value)) {
    return("null")
  }
  encoded <- format(
    value,
    digits = 17L,
    scientific = NA,
    trim = TRUE,
    decimal.mark = "."
  )
  if (!grepl(
    "^-?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?$",
    encoded,
    perl = TRUE
  )) {
    stop("Base R produced a number that is not valid JSON.", call. = FALSE)
  }
  encoded
}

# Encode only the bounded, plain lists produced by rho.bridge. Deliberately
# rejecting classed and language objects prevents accidental traversal of an
# arbitrary scientific object graph.
rho_json_encode <- function(value, max_depth = 64L, max_values = 100000L) {
  max_depth <- as.integer(max_depth)
  max_values <- as.integer(max_values)
  if (
    length(max_depth) != 1L || is.na(max_depth) || max_depth < 0L ||
      length(max_values) != 1L || is.na(max_values) || max_values < 1L
  ) {
    stop("JSON encoder limits must be finite positive integers.", call. = FALSE)
  }

  values_seen <- 0L

  encode_value <- function(item, depth) {
    values_seen <<- values_seen + 1L
    if (values_seen > max_values) {
      stop("Structured bridge result exceeds the JSON value limit.", call. = FALSE)
    }
    if (depth > max_depth) {
      stop("Structured bridge result exceeds the JSON nesting limit.", call. = FALSE)
    }
    if (is.null(item)) {
      return("null")
    }
    if (is.object(item)) {
      stop(
        sprintf(
          "Cannot JSON encode classed object <%s>; bridge results must be plain and bounded.",
          paste(class(item), collapse = "/")
        ),
        call. = FALSE
      )
    }

    encode_atomic <- function(encode_scalar) {
      if (!length(item)) {
        return("[]")
      }
      remaining_values <- max_values - values_seen
      if (length(item) - 1L > remaining_values) {
        stop("Structured bridge result exceeds the JSON value limit.", call. = FALSE)
      }
      values_seen <<- values_seen + length(item) - 1L
      encoded <- vapply(
        seq_along(item),
        function(index) encode_scalar(item[[index]]),
        character(1),
        USE.NAMES = FALSE
      )
      if (length(encoded) == 1L) encoded else paste0("[", paste(encoded, collapse = ","), "]")
    }

    if (is.logical(item)) {
      return(encode_atomic(function(scalar) {
        if (is.na(scalar)) "null" else if (scalar) "true" else "false"
      }))
    }
    if (is.integer(item)) {
      return(encode_atomic(function(scalar) {
        if (is.na(scalar)) "null" else as.character(scalar)
      }))
    }
    if (is.double(item)) {
      return(encode_atomic(json_number))
    }
    if (is.character(item)) {
      return(encode_atomic(function(scalar) {
        if (is.na(scalar)) "null" else json_escape_string(scalar)
      }))
    }
    if (is.list(item)) {
      item_names <- names(item)
      if (is.null(item_names)) {
        if (!length(item)) {
          return("[]")
        }
        encoded <- vapply(
          item,
          encode_value,
          character(1),
          depth = depth + 1L,
          USE.NAMES = FALSE
        )
        return(paste0("[", paste(encoded, collapse = ","), "]"))
      }
      if (anyNA(item_names) || any(!nzchar(item_names))) {
        stop("JSON object names must be non-missing and non-empty.", call. = FALSE)
      }
      if (anyDuplicated(item_names)) {
        stop("JSON object names must be unique.", call. = FALSE)
      }
      if (!length(item)) {
        return("{}")
      }
      encoded <- vapply(seq_along(item), function(index) {
        paste0(
          json_escape_string(item_names[[index]]),
          ":",
          encode_value(item[[index]], depth + 1L)
        )
      }, character(1), USE.NAMES = FALSE)
      return(paste0("{", paste(encoded, collapse = ","), "}"))
    }

    stop(
      sprintf(
        "Cannot JSON encode R type <%s>; bridge results must contain only plain JSON values.",
        typeof(item)
      ),
      call. = FALSE
    )
  }

  enc2utf8(encode_value(value, 0L))
}

compact_text <- function(x, max_chars = 4000L) {
  value <- paste(x, collapse = "\n")
  if (nchar(value, type = "chars") <= max_chars) {
    return(value)
  }
  paste0(substr(value, 1L, max_chars), "\n... [truncated]")
}

safe_call_text <- function(call) {
  tryCatch(
    paste(deparse(call, width.cutoff = 200L), collapse = " "),
    error = function(e) "<unavailable>"
  )
}

#' Return the Last Structured Workspace Execution
#' @export
rho_get_last_execution <- function() {
  .rho_bridge_state$last_execution
}
