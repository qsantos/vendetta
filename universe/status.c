#include "status.h"

#include <stdlib.h>

void kindOf_status_init(kindOf_status_t* s)
{
	s->name = NULL;
}

void kindOf_status_exit(kindOf_status_t* s)
{
	free(s->name);
}
