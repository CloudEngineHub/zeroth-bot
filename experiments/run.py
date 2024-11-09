from robot import Robot, RobotConfig
import pygame
import time
import math
import threading
import multiprocessing as mp
import onnxruntime as ort
from model import inference
import os


def state_stand(robot: Robot) -> bool:
    print("Standing")
    positions = {
        "left_hip_pitch": 0.0, "right_hip_pitch": 0.0,
        "left_hip_yaw": 0.0, "right_hip_yaw": 0.0,
        "left_hip_roll": 0.0, "right_hip_roll": 0.0,
        "left_knee_pitch": 0.0, "right_knee_pitch": 0.0,
        "left_ankle_pitch": 0.0, "right_ankle_pitch": 0.0,
        "left_shoulder_pitch": 0.0, "right_shoulder_pitch": 0.0,
        "left_shoulder_yaw": 0.0, "right_shoulder_yaw": 0.0,
        "left_elbow_yaw": 0.0, "right_elbow_yaw": 0.0
    }
    robot.set_desired_positions(positions)
    return True


def state_walk(robot: Robot, stop_event: threading.Event) -> bool:
    print("Walking")

    current_dir = os.path.dirname(os.path.abspath(__file__))
    model_path = os.path.join(current_dir, "..", "sim", "examples", "walking_micro.onnx")
    if not os.path.isfile(model_path):
        print(f"Model file not found at {model_path}")
        return False
    policy = ort.InferenceSession(model_path)

    data_queue = mp.Queue()

    inference_thread = threading.Thread(
        target=inference, args=(policy, robot, data_queue, stop_event)
    )
    inference_thread.start()

    return True


def state_forward_recovery(robot: Robot) -> bool:
    print("Forward recovery")
    
    # Initialize all joints to 0
    initial_positions = {joint.name: 0.0 for joint in robot.joints}
    robot.set_desired_positions(initial_positions)

    # Getting feet on the ground
    robot.set_desired_positions({
        "left_hip_pitch": 30.0,
        "right_hip_pitch": -30.0,
        "left_knee_pitch": 50.0,
        "right_knee_pitch": -50.0,
        "left_ankle_pitch": -30.0,
        "right_ankle_pitch": 30.0,
    })

    # 90 degree position
    robot.set_desired_positions({
        "left_shoulder_pitch": 30.0,
        "right_shoulder_pitch": -30.0,
        "left_shoulder_yaw": -20.0,
        "right_shoulder_yaw": 20.0,
        "left_elbow_yaw": -60.0,
        "right_elbow_yaw": 60.0,
        "left_hip_pitch": 30.0,
        "right_hip_pitch": -30.0,
        "left_knee_pitch": 70.0,
        "right_knee_pitch": -70.0,
        "left_ankle_pitch": -30.0,
        "right_ankle_pitch": 30.0,
    })

    # Prep Position
    robot.set_desired_positions({
        "left_shoulder_pitch": 30.0,
        "right_shoulder_pitch": -30.0,
        "left_shoulder_yaw": 20.0,
        "right_shoulder_yaw": -20.0,
        "left_elbow_yaw": -20.0,
        "right_elbow_yaw": 20.0,
        "left_hip_pitch": 30.0,
        "right_hip_pitch": -30.0,
        "left_knee_pitch": 70.0,
        "right_knee_pitch": -70.0,
        "left_ankle_pitch": 0.0,
        "right_ankle_pitch": 0.0,
    })

    time.sleep(1)

    robot.set_desired_positions({
        "left_shoulder_pitch": 120.0,
        "right_shoulder_pitch": -120.0,
        "left_hip_pitch": 80.0,
        "right_hip_pitch": -80.0,
    })

    robot.set_desired_positions({
        "left_knee_pitch": 90.0,
        "right_knee_pitch": -90.0,
        "left_ankle_pitch": 40.0,
        "right_ankle_pitch": -40.0,
    })

    robot.set_desired_positions({
        "left_elbow_yaw": 0.0,
        "right_elbow_yaw": 0.0,
        "left_knee_pitch": 90.0,
        "right_knee_pitch": -90.0,
        "left_ankle_pitch": 40.0,
        "right_ankle_pitch": -40.0,
    })

    time.sleep(2)

    # Box Position
    robot.set_desired_positions({
        "left_shoulder_yaw": -40.0,
        "right_shoulder_yaw": 40.0,
        "left_ankle_pitch": 90.0,
        "right_ankle_pitch": -90.0,
    })

    time.sleep(2)

    # Tilting torso 1
    robot.set_desired_positions({
        "left_shoulder_pitch": 120.0,
        "right_shoulder_pitch": -120.0,
        "left_shoulder_yaw": -40.0,
        "right_shoulder_yaw": 40.0,
        "left_elbow_yaw": 0.0,
        "right_elbow_yaw": -0.0,
        "left_hip_pitch": 50.0,
        "right_hip_pitch": -50.0,
        "left_knee_pitch": 60.0,
        "right_knee_pitch": -60.0,
        "left_ankle_pitch": 60.0,
        "right_ankle_pitch": -60.0,
    })

    time.sleep(2)

        # Tilting torso 2
    robot.set_desired_positions({
        "left_shoulder_pitch": 120.0,
        "right_shoulder_pitch": -120.0,
        "left_shoulder_yaw": -40.0,
        "right_shoulder_yaw": 40.0,
        "left_elbow_yaw": 0.0,
        "right_elbow_yaw": -0.0,
        "left_hip_pitch": 22.0,
        "right_hip_pitch": -22.0,
        "left_knee_pitch": 50.0,
        "right_knee_pitch": -50.0,
        "left_ankle_pitch": 50.0,
        "right_ankle_pitch": -50.0,
    })

    time.sleep(2)

        # Tilting torso 3
    robot.set_desired_positions({
        "left_shoulder_pitch": 120.0,
        "right_shoulder_pitch": -120.0,
        "left_shoulder_yaw": -40.0,
        "right_shoulder_yaw": 40.0,
        "left_elbow_yaw": 0.0,
        "right_elbow_yaw": -0.0,
        "left_hip_pitch": 10.0,
        "right_hip_pitch": -10.0,
        "left_knee_pitch": 30.0,
        "right_knee_pitch": -30.0,
        "left_ankle_pitch": 30.0,
        "right_ankle_pitch": -30.0,
    })

    time.sleep(2)

        # Standing Straight
    robot.set_desired_positions({
        "left_shoulder_pitch": 0.0,
        "right_shoulder_pitch": -0.0,
        "left_shoulder_yaw": -0.0,
        "right_shoulder_yaw": 0.0,
        "left_elbow_yaw": 0.0,
        "right_elbow_yaw": -0.0,
        "left_hip_pitch": 5.0,
        "right_hip_pitch": -5.0,
        "left_knee_pitch": 0.0,
        "right_knee_pitch": -0.0,
        "left_ankle_pitch": 0.0,
        "right_ankle_pitch": -0.0,
    })

    # Set torque to 20 for all servos
    for joint in robot.joints:
        robot.hal.servo.set_torque([(joint.servo_id, 20.0)])
    
    time.sleep(1)

    return True

def state_backward_recovery(robot : Robot) -> bool:
    print("Backward recovery")
    robot.set_desired_positions({joint.name: 0.0 for joint in robot.joints})

    return True

def state_drop_forward(robot : Robot) -> bool:
    print("Drop forward")
    robot.set_desired_positions({joint.name: 0.0 for joint in robot.joints})

    return True

def state_pushups(robot: Robot) -> bool:
    print("Pushups - Press 'x' to stop")
    robot.set_desired_positions({joint.name: 0.0 for joint in robot.joints})
    
    # Start Position 1
    robot.set_desired_positions({
        "left_shoulder_pitch": 90.0,
        "right_shoulder_pitch": -90.0,
        "left_shoulder_yaw": 90.0,
        "right_shoulder_yaw": -90.0,
        "left_elbow_yaw": 0.0,
        "right_elbow_yaw": 0.0,
        "left_hip_pitch": 10.0,
        "right_hip_pitch": -10.0,
        "left_hip_roll": 0.0,
        "right_hip_roll": 0.0,
        "left_hip_yaw": -5.0,
        "right_hip_yaw": 5.0,
        "left_knee_pitch": 5.0,
        "right_knee_pitch": -5.0,
        "left_ankle_pitch": -100.0,
        "right_ankle_pitch": 100.0,
    })

    time.sleep(1)

    # Start Position 2
    robot.set_desired_positions({
        "left_shoulder_pitch": 90.0,
        "right_shoulder_pitch": -90.0,
        "left_shoulder_yaw": 90.0,
        "right_shoulder_yaw": -90.0,
        "left_elbow_yaw": 90.0,
        "right_elbow_yaw": -90.0,
        "left_hip_pitch": 10.0,
        "right_hip_pitch": -10.0,
        "left_hip_roll": 0.0,
        "right_hip_roll": 0.0,
        "left_hip_yaw": -5.0,
        "right_hip_yaw": 5.0,
        "left_knee_pitch": 5.0,
        "right_knee_pitch": -5.0,
        "left_ankle_pitch": -80.0,
        "right_ankle_pitch": 80.0,
    })

    # Set torque for shoulder yaw and elbow yaw to 50
    robot.hal.servo.set_torque([
        (15, 50.0),  # left_shoulder_yaw
        (12, 50.0),  # right_shoulder_yaw
        (16, 50.0),  # left_elbow_yaw
        (11, 50.0)   # right_elbow_yaw
    ])

    time.sleep(1)

    running = True
    while running:
        for event in pygame.event.get():
            if event.type == pygame.KEYDOWN and event.key == pygame.K_x:
                running = False
                return True
        
        # Push up 1
        robot.set_desired_positions({
            "left_shoulder_pitch": 90.0,
            "right_shoulder_pitch": -90.0,
            "left_shoulder_yaw": -40.0,
            "right_shoulder_yaw": 40.0,
            "left_elbow_yaw": 0.0,
            "right_elbow_yaw": 0.0,
            "left_hip_pitch": 10.0,
            "right_hip_pitch": -10.0,
            "left_hip_roll": 0.0,
            "right_hip_roll": 0.0,
            "left_hip_yaw": -5.0,
            "right_hip_yaw": 5.0,
            "left_knee_pitch": 5.0,
            "right_knee_pitch": -5.0,
            "left_ankle_pitch": -70.0,
            "right_ankle_pitch": 70.0,
        })
        time.sleep(1)

        # Push Down 1
        robot.set_desired_positions({
            "left_shoulder_pitch": 90.0,
            "right_shoulder_pitch": -90.0,
            "left_shoulder_yaw": 90.0,
            "right_shoulder_yaw": -90.0,
            "left_elbow_yaw": 90.0,
            "right_elbow_yaw": -90.0,
            "left_hip_pitch": 10.0,
            "right_hip_pitch": -10.0,
            "left_hip_roll": 0.0,
            "right_hip_roll": 0.0,
            "left_hip_yaw": -5.0,
            "right_hip_yaw": 5.0,
            "left_knee_pitch": 5.0,
            "right_knee_pitch": -5.0,
            "left_ankle_pitch": -80.0,
            "right_ankle_pitch": 80.0,
        })
        time.sleep(1)

    # Set torque to 20 for all servos before exiting
    for joint in robot.joints:
        robot.hal.servo.set_torque([(joint.servo_id, 20.0)])
    
    return True

def state_wave(robot: Robot) -> bool:
    print("Waving")

    initial_positions = {
        "left_shoulder_yaw": 0.0,
        "left_shoulder_pitch": 0.0,
        "left_elbow_yaw": 0.0,
    }
    robot.set_desired_positions(initial_positions)
    time.sleep(0.5)

    wave_up_positions = {
        "left_shoulder_pitch": 0.0,
        "left_shoulder_yaw": 150.0,
    }
    robot.set_desired_positions(wave_up_positions)
    time.sleep(0.5)

    for _ in range(6):
        wave_out = {"left_elbow_yaw": -90.0}
        robot.set_desired_positions(wave_out)
        time.sleep(0.3)

        wave_in = {"left_elbow_yaw": -45.0}
        robot.set_desired_positions(wave_in)
        time.sleep(0.3)

    robot.set_desired_positions(initial_positions)
    time.sleep(0.5)

    return True


def main():
    robot = Robot()
    try:
        robot.initialize()
        state_stand(robot)

        pygame.init()
        screen = pygame.display.set_mode((400, 300))
        pygame.display.set_caption("Robot Control")

        print(
            "Press 'w' to walk, 'space' to stand, 'q' to wave, '1' for forward recovery, '2' for backward recovery, 'escape' to quit"
        )

        running = True
        stop_event = threading.Event()
        current_thread = None

        while running:
            try:
                for event in pygame.event.get():
                    if event.type == pygame.QUIT:
                        running = False
                    elif event.type == pygame.KEYDOWN:
                        try:
                            # Stop current action if any
                            if current_thread is not None:
                                stop_event.set()
                                current_thread.join()
                                stop_event.clear()
                                current_thread = None

                            if event.key == pygame.K_w:
                                # Start walking inference in a new thread
                                current_dir = os.path.dirname(os.path.abspath(__file__))
                                model_path = os.path.join(current_dir, "..", "sim", "examples", "walking_micro.onnx")
                                if not os.path.isfile(model_path):
                                    print(f"Model file not found at {model_path}")
                                    continue  # Skip if model not found
                                policy = ort.InferenceSession(model_path)
                                data_queue = mp.Queue()
                                current_thread = threading.Thread(
                                    target=inference, args=(policy, robot, data_queue, stop_event)
                                )
                                current_thread.start()
                            elif event.key == pygame.K_SPACE:
                                state_stand(robot)
                            elif event.key == pygame.K_q:
                                state_wave(robot)
                            elif event.key == pygame.K_1:
                                state_forward_recovery(robot)
                            elif event.key == pygame.K_2:
                                state_backward_recovery(robot)
                            elif event.key == pygame.K_ESCAPE:
                                running = False
                        except Exception as e:
                            print(f"Error during state execution: {e}")
            except KeyboardInterrupt:
                print("\nCtrl+C detected, shutting down gracefully...")
                break

    except Exception as e:
        print(f"Error during robot operation: {e}")
    finally:
        # Ensure any running threads are stopped
        if current_thread is not None:
            stop_event.set()
            current_thread.join()
            stop_event.clear()
            current_thread = None
        try:
            robot.disable_motors()
            print("Motors disabled")
        except:
            print("Error disabling motors")
        pygame.quit()


if __name__ == "__main__":
    main()
