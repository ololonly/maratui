ifeq ($(OS),Windows_NT)
  SIM_CMD = cargo simwin
  SIMHA_CMD = cargo simwinHA
else
  UNAME_S := $(shell uname -s)
  ifeq ($(UNAME_S),Darwin)
    SIM_CMD = cargo simmac
    SIMHA_CMD = cargo simmacHA
  else
    SIM_CMD = cargo sim
    SIMHA_CMD = cargo simha
  endif
endif

sim:
	$(SIM_CMD)

sim-ha:
	$(SIMHA_CMD)

flash-ha:
	cargo run --release --features home-assistant
