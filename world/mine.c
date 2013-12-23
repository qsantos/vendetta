#include "mine.h"

void mine_init(mine_t* m, kindOf_mine_t* t)
{
	m->o.t = O_MINE;
	m->o.x = 0;
	m->o.y = 0;
	m->o.w = 32;
	m->o.h = 32;
	m->t = t;
}
