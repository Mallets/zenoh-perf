generate_stats <- function (data_path, out_path, date) {
  sizes <- c(8)
  for(i in 1:21) {
    sizes = append(sizes, sizes[length(sizes)]*2)
  }
  print(sizes)
  td <- NULL
  for (s in sizes) {
    n = gettextf("%s/%s.txt",data_path, s)
    d = read.delim(n)
    sd = summary(d[,1])
    td = rbind(td, sd)
  }
  rownames(td) <- sizes
  sname = gettextf("%s/%s-%s.csv",out_path, date, "stats")
  write.csv2(td, file=sname)
}

args = commandArgs(trailingOnly=TRUE)
print(args)
generate_stats(args[1], args[2], args[3])

