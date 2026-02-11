ifeq ($(OS),Windows_NT)
  SIM_CMD = cargo simwin
else
  UNAME_S := $(shell uname -s)
  ifeq ($(UNAME_S),Darwin)
    SIM_CMD = cargo simmac
  else
    SIM_CMD = cargo sim
  endif
endif

sim:
	$(SIM_CMD)
