# NATS-go version 1.10

all:
	mkdir -p ./target
	go build -o target/nats_pub_thr throughput/nats_pub_thr.go
	go build -o target/nats_sub_thr throughput/nats_sub_thr.go
	go build -o target/nats_ping latency/nats_ping.go
	go build -o target/nats_pong latency/nats_pong.go
	go build -o target/nats_overhead overhead/nats_overhead.go
	go build -o target/nats_eval query/nats_eval.go
	go build -o target/nats_query_thr query/nats_query_thr.go
	go build -o target/nats_query query/nats_query.go

clean:
	rm -rf target