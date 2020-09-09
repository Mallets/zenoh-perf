library(ggplot2)

zcreate_series <- function(stats_path, out_dir, date) {
  # looks for all the stats file in the provided path and creates a consolidated view 
  # that shows the evolution over the various tests.
  
  meds <- NULL
  mins <- NULL
  maxs <- NULL
  
  fs <- list.files(stats_path)
  for (f in fs) {
    print("Reading:")
    print(f)
    name = gettextf("%s/%s",stats_path, f)
    data = read.csv2(name)
    meds <- cbind(meds, data[, 4])
    mins <- cbind(mins, data[, 2])
    maxs <- cbind(maxs, data[, 7])
  }
  meds_dname <- gettextf("%s/%s-%s.csv",out_dir, date, "meds")
  mins_dname <- gettextf("%s/%s-%s.csv",out_dir, date, "mins")
  maxs_dname <- gettextf("%s/%s-%s.csv",out_dir, date, "maxs")
  write.csv2(meds, file = meds_dname)
  write.csv2(mins, file = mins_dname)
  write.csv2(maxs, file = maxs_dname)
  
  idx <- seq(1, dim(meds)[1])
  pload <- 8
  X <- t(seq(1, dim(meds)[2]))
  # This is not a mistake... Just a strange  R behaviour...
  X <- t(X)
  for (i in idx) {
    m3_iname = gettextf("%s/%s-%d-%s.png",out_dir, date, pload, "m3")
    meds_iname = gettextf("%s/%s-%d-%s.png",out_dir, date, pload, "meds")
    YMed <- meds[i, 1:dim(meds)[2]]
    YMed <- t(YMed)
    YMed <- t(YMed)
    rownames(YMed) <- NULL
    colnames(YMed) <- NULL
    D <- cbind(X, YMed)
    colnames(D) <- factor(c("Obs", "Value"))
    FDMed = as.data.frame(D)
    # png(meds_iname)
    # g <- ggplot(FDMed, aes(x=Obs, y=Value)) + geom_line() + geom_point()
    # print(g)
    # dev.off()
    
    mins_iname <- gettextf("%s/%s-%d-%s.png",out_dir, date, pload, "mins")
    YMin <- mins[i, 1:dim(mins)[2]]
    YMin <- t(YMin)
    YMin <- t(YMin)
    names(YMin) <- NULL
    D <- cbind(X, YMin)
    colnames(D) <- factor(c("Obs", "Value"))
    FDMin = as.data.frame(D)
    # png(mins_iname)
    # g <- ggplot(FDMin, aes(x=Obs, y=Value)) + geom_line() + geom_point()
    # print(g)
    # dev.off()
    
    maxs_iname <- gettextf("%s/%s-%d-%s.png",out_dir, date, pload, "maxs")
    YMax <- maxs[i, 1:dim(maxs)[2]]
    YMax <- t(YMax)
    YMax <- t(YMax)
    names(YMax) <- NULL
    D <- cbind(X, YMax)
    colnames(D) <- factor(c("Obs", "Value"))
    FDMax = as.data.frame(D)
    # png(maxs_iname)
    # g <- ggplot(FDMax, aes(x=Obs, y=Value)) + geom_line() + geom_point()
    # print(g)
    # dev.off()
    
    png(m3_iname)
    g <- ggplot(FDMed, aes(x=Obs, y=Value)) + geom_line(color = "green") + geom_point(color = "green") +
      geom_line(data=FDMin, color="blue") + geom_point(data=FDMin, color = "green") + 
      geom_line(data=FDMax, color="red") + geom_point(data=FDMax, color = "red")
    print(g)
    dev.off()
    
    pload <- pload * 2
  }
  

}


args <- commandArgs(trailingOnly=TRUE)
zcreate_series(args[1], args[2], args[3])
