#include <stdlib.h>
#include <SFML/Graphics.h>

#include "world.h"
#include "draw.h"

int main(void)
{
	sfVideoMode mode = {800, 600, 32};
	sfRenderWindow* window = sfRenderWindow_create(mode, "SFML window", sfResize | sfClose, NULL);
	if (window == NULL)
		exit(1);

	sfTexture* texture = sfTexture_createFromFile("character.png", NULL);
	if (texture == NULL)
		exit(1);

	world_t world;
	world_init(&world);

	character_t* player = &world.characters[0];

	sfClock* clock = sfClock_create();
	while (sfRenderWindow_isOpen(window))
	{
		float duration = sfTime_asSeconds(sfClock_getElapsedTime(clock));
		sfClock_restart(clock);

		sfEvent event;
		while (sfRenderWindow_pollEvent(window, &event))
		{
			if (event.type == sfEvtClosed)
				sfRenderWindow_close(window);
			else if (event.type == sfEvtKeyPressed)
			{
				sfKeyCode k = event.key.code;
				if (k == sfKeyEscape)
					sfRenderWindow_close(window);
			}
		}

		player->go_x = 100 * (sfKeyboard_isKeyPressed(sfKeyRight) - 2*sfKeyboard_isKeyPressed(sfKeyLeft));
		player->go_y = 100 * (sfKeyboard_isKeyPressed(sfKeyDown)  - 2*sfKeyboard_isKeyPressed(sfKeyUp));

		world_doRound(&world, duration);

		sfRenderWindow_clear(window, sfBlack);
		draw_world(window, &world);
		sfRenderWindow_display(window);
	}

	sfClock_destroy(clock);
//	sfSprite_destroy(sprite); // TODO
//	sfTexture_destroy(texture);
	sfRenderWindow_destroy(window);
	return 0;
}
