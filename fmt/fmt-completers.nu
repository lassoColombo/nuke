
export def output [] { [full wide compact] }

export def output-no-wide [] { output | where {$in != wide} }

export def output-no-full [] { output | where {$in != full} }

