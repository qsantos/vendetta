#include "mine.h"

#include <string.h>

void mine_init(mine_t* m)
{
	memset(m, 0, sizeof(mine_t));
	m->o.t = O_MINE;
}
