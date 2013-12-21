#ifndef MINE_H
#define MINE_H

#include "object.h"

typedef struct mine mine_t;

struct mine
{
	object_t o;
};

void mine_init(mine_t* m);

#endif
