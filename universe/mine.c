#include "mine.h"

#include <stdlib.h>

void kindOf_mine_exit(kindOf_mine_t* m)
{
	free(m->name);
}
