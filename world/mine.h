#ifndef W_MINE_H
#define W_MINE_H

#include "object.h"
#include "../universe/mine.h"

typedef struct mine mine_t;

struct mine
{
	object_t o;

	kindOf_mine_t* t;
};

void mine_init(mine_t* m, kindOf_mine_t* t);

#endif
